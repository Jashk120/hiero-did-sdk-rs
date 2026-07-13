//! End-to-end happy-path integration tests against a local Hedera node.
//!
//! # Prerequisites
//! - Start the local node: `docker compose up -d` from the repo root.
//! - Configure credentials in `.env.local` or `.env`:
//!     HEDERA_ACCOUNT_ID=0.0.2
//!     HEDERA_PRIVATE_KEY=<DER key>
//!     HEDERA_NETWORK=local
//!
//! If the env is missing or the local node is unreachable, tests silently
//! skip rather than failing. The health-check gate in the common harness
//! will print a clear message to stderr explaining the skip.

mod common;
use common::local_node::{
    MockExternalSigner,
    setup,
};
use hiero_did_sdk::core::did::Network;
use hiero_did_sdk::registrar::{
    AddService,
    CsmPrepareOptions,
    DIDUpdateOperation,
};
use hiero_did_sdk::resolver::MirrorNodeClient;
use serial_test::serial;

// ---------------------------------------------------------------------------
// Standard lifecycle
// ---------------------------------------------------------------------------

/// Creates a DID, resolves it, updates it, resolves again, then deactivates.
///
/// This is the primary end-to-end regression that must always pass.
#[tokio::test]
#[serial]
async fn lifecycle_create_update_deactivate_resolve() {
    let Some(ctx) = setup() else { return };
    let mirror = MirrorNodeClient::for_local();

    // 1. Create
    let create_result =
        ctx.sdk.create_did(None, Network::Local, None).await.expect("create_did failed");
    let did = create_result.did.clone();
    println!("[lifecycle] created DID: {did}");

    mirror.wait_for_mirror(&did.topic_id, 30).await.expect("Mirror wait post-create failed");

    // 2. Resolve — should find a live, non-deactivated document
    let resolution = ctx
        .sdk
        .resolve_did(&did.to_string(), None)
        .await
        .expect("resolve_did (post-create) failed");
    assert_eq!(resolution.did_document.id, did.to_string());
    assert!(
        !resolution.did_document_metadata.deactivated.unwrap_or(false),
        "DID should not be deactivated yet"
    );

    // 3. Update — add a service
    let update_result = ctx
        .sdk
        .update_did(
            None,
            did.clone(),
            &create_result.private_key_bytes,
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{}#vcs", did),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com/did".to_string(),
            })],
        )
        .await
        .expect("update_did failed");
    assert_eq!(update_result.operations_applied, 1);

    mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Mirror wait post-update failed");

    // 4. Resolve post-update — service should be present
    let resolution_after_update = ctx
        .sdk
        .resolve_did(&did.to_string(), None)
        .await
        .expect("resolve_did (post-update) failed");
    let services = resolution_after_update.did_document.service.unwrap_or_default();
    assert!(
        services.iter().any(|s| s.id.ends_with("#vcs")),
        "service should be present after update"
    );

    // 5. Deactivate
    ctx.sdk
        .deactivate_did(None, did.clone(), &create_result.private_key_bytes)
        .await
        .expect("deactivate_did failed");

    mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Mirror wait post-deactivate failed");

    // 6. Resolve post-deactivate — must return deactivated flag in metadata
    let resolution_after_deactivate = ctx
        .sdk
        .resolve_did(&did.to_string(), None)
        .await
        .expect("resolve_did (post-deactivate) failed");
    assert!(
        resolution_after_deactivate.did_document_metadata.deactivated.unwrap_or(false),
        "resolved DID should be marked deactivated"
    );
}

/// Create DID, update with multiple operations in one call, confirm all applied.
#[tokio::test]
#[serial]
async fn update_did_applies_multiple_operations() {
    let Some(ctx) = setup() else { return };
    let mirror = MirrorNodeClient::for_local();

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create_did failed");
    let did = create.did.clone();

    mirror.wait_for_mirror(&did.topic_id, 30).await.expect("Mirror wait post-create failed");

    let result = ctx
        .sdk
        .update_did(
            None,
            did.clone(),
            &create.private_key_bytes,
            vec![
                DIDUpdateOperation::AddService(AddService {
                    id: format!("{}#svc-1", did),
                    service_type: "LinkedDomains".to_string(),
                    service_endpoint: "https://example.com".to_string(),
                }),
                DIDUpdateOperation::AddService(AddService {
                    id: format!("{}#svc-2", did),
                    service_type: "DIDCommMessaging".to_string(),
                    service_endpoint: "https://example.com/msg".to_string(),
                }),
            ],
        )
        .await
        .expect("multi-op update failed");

    assert_eq!(result.operations_applied, 2);

    mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Mirror wait post-update failed");

    let resolved = ctx.sdk.resolve_did(&did.to_string(), None).await.expect("resolve_did failed");
    let services = resolved.did_document.service.unwrap_or_default();
    assert_eq!(services.len(), 2);
}

/// Create DID and remove a service that was previously added.
#[tokio::test]
#[serial]
async fn update_did_add_then_remove_service() {
    let Some(ctx) = setup() else { return };
    let mirror = MirrorNodeClient::for_local();

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();
    let svc_id = format!("{}#endpoint", did);

    mirror.wait_for_mirror(&did.topic_id, 30).await.expect("Mirror wait post-create failed");

    // Add service
    ctx.sdk
        .update_did(
            None,
            did.clone(),
            &create.private_key_bytes,
            vec![DIDUpdateOperation::AddService(AddService {
                id: svc_id.clone(),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
            })],
        )
        .await
        .expect("add service failed");

    mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Mirror wait post-add failed");

    // Remove service
    ctx.sdk
        .update_did(
            None,
            did.clone(),
            &create.private_key_bytes,
            vec![DIDUpdateOperation::RemoveService(hiero_did_sdk::registrar::RemoveService {
                id: svc_id.clone(),
            })],
        )
        .await
        .expect("remove service failed");
    let resolved = ctx
        .sdk
        .resolve_did_until(&did.to_string(), 30, |r| {
            r.did_document.service.as_ref().map_or(true, |s| s.iter().all(|x| x.id != svc_id))
        })
        .await
        .expect("service removal did not converge within timeout");
    let services = resolved.did_document.service.unwrap_or_default();
    assert!(
        services.iter().all(|s| s.id != svc_id),
        "removed service should not appear in resolved document"
    );
}

// ---------------------------------------------------------------------------
// External signer (mock) — _with_signer HCS write path
//
// SCOPE: proves the SDK wires external signatures into HCS transactions.
// Does NOT prove: Vault auth, token handling, HSM encoding compatibility.
// See graduation PR for real-Vault follow-up suite.
// ---------------------------------------------------------------------------

/// vault_signer_hcs_path_mock_signer — proves _with_signer plumbing.
///
/// This test uses a mock signer (not a real Vault connection).
/// It verifies that the SDK correctly accepts an externally-provided signature
/// and writes to HCS. See common::local_node::MockExternalSigner for scope.
#[tokio::test]
#[serial]
async fn vault_signer_hcs_path_mock_signer_create() {
    let Some(ctx) = setup() else { return };

    let mock_signer = MockExternalSigner::new();

    let result = ctx
        .sdk
        .create_did_with_signer(None, Network::Local, None, &mock_signer)
        .await
        .expect("create_did_with_signer (mock signer) failed");

    println!("[vault_mock] created DID: {}", result.did);
    assert!(!result.did.to_string().is_empty());
}

/// vault_signer_hcs_path_mock_signer_update — update via external signer.
#[tokio::test]
#[serial]
async fn vault_signer_hcs_path_mock_signer_update() {
    let Some(ctx) = setup() else { return };

    let mock_signer = MockExternalSigner::new();
    let create = ctx
        .sdk
        .create_did_with_signer(None, Network::Local, None, &mock_signer)
        .await
        .expect("create failed");

    let did = create.did.clone();
    let result = ctx
        .sdk
        .update_did_with_signer(
            None,
            did.clone(),
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{}#svc-mock", did),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com/mock".to_string(),
            })],
            &mock_signer,
        )
        .await
        .expect("update_did_with_signer (mock signer) failed");

    assert_eq!(result.operations_applied, 1);
}

/// vault_signer_hcs_path_mock_signer_deactivate — deactivate via external signer.
#[tokio::test]
#[serial]
async fn vault_signer_hcs_path_mock_signer_deactivate() {
    let Some(ctx) = setup() else { return };

    let mock_signer = MockExternalSigner::new();
    let create = ctx
        .sdk
        .create_did_with_signer(None, Network::Local, None, &mock_signer)
        .await
        .expect("create failed");

    ctx.sdk
        .deactivate_did_with_signer(None, create.did.clone(), &mock_signer)
        .await
        .expect("deactivate_did_with_signer (mock signer) failed");
}

// ---------------------------------------------------------------------------
// CSM (Client-Side Message) flow — prepare → sign → submit
// ---------------------------------------------------------------------------

/// Full CSM create loop: prepare, sign offline, submit, verify on-chain.
#[tokio::test]
#[serial]
async fn csm_create_prepare_sign_submit() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();

    // 1. Prepare — pure state machine, no network
    let signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes, None)
        .await
        .expect("prepare_create_did_csm failed");

    assert_eq!(signing_req.operation, "create");
    assert!(!signing_req.message_bytes.is_empty());

    // 2. Sign offline (simulating external signing)
    let signature = key.sign(&signing_req.message_bytes);

    // 3. Convert to submit request
    let submit_req =
        signing_req.into_submit_request(signature).expect("into_submit_request failed");

    // 4. Submit to HCS
    let result = ctx
        .sdk
        .submit_create_did_csm(None, submit_req)
        .await
        .expect("submit_create_did_csm failed");

    assert_eq!(result.operation, "create");
    assert!(!result.did.is_empty());
    println!("[csm_create] submitted DID: {}", result.did);
}

/// Full CSM update loop.
#[tokio::test]
#[serial]
async fn csm_update_prepare_sign_submit() {
    let Some(ctx) = setup() else { return };

    // First create a DID (standard path so we have a known key)
    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");

    let did = create.did.clone();

    // Prepare the update CSM batch (no network call)
    let batch_req = ctx
        .sdk
        .prepare_update_did_csm(
            did.clone(),
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{}#csm-svc", did),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://csm.example.com".to_string(),
            })],
        )
        .await
        .expect("prepare_update_did_csm failed");

    assert_eq!(batch_req.requests.len(), 1);
    assert_eq!(batch_req.operation, "update");

    // Sign each request offline
    let key =
        hiero_sdk::PrivateKey::from_bytes(&create.private_key_bytes).expect("invalid key bytes");
    let signatures: Vec<Vec<u8>> =
        batch_req.requests.iter().map(|r| key.sign(&r.message_bytes)).collect();

    let submit_req = batch_req.into_submit_request(signatures).expect("into_submit_request failed");

    let result = ctx
        .sdk
        .submit_update_did_csm(None, submit_req)
        .await
        .expect("submit_update_did_csm failed");

    assert_eq!(result.operations_applied, 1);
}

/// Full CSM deactivate loop.
#[tokio::test]
#[serial]
async fn csm_deactivate_prepare_sign_submit() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    // Prepare — no network
    let signing_req = ctx
        .sdk
        .prepare_deactivate_did_csm(did.clone())
        .await
        .expect("prepare_deactivate_did_csm failed");

    assert_eq!(signing_req.operation, "delete");

    // Sign offline
    let key = hiero_sdk::PrivateKey::from_bytes(&create.private_key_bytes).unwrap();
    let signature = key.sign(&signing_req.message_bytes);

    let submit_req =
        signing_req.into_submit_request(signature).expect("into_submit_request failed");

    ctx.sdk
        .submit_deactivate_did_csm(None, submit_req)
        .await
        .expect("submit_deactivate_did_csm failed");
}

/// CSM prepare with custom options (expiry timestamp) compiles and returns correct structure.
#[tokio::test]
#[serial]
async fn csm_create_with_options_sets_expiry() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();

    let far_future_unix: i64 = 9999999999;
    let options = CsmPrepareOptions { expires_at_unix: Some(far_future_unix) };

    let signing_req = ctx
        .sdk
        .prepare_create_did_csm_with_options(None, Network::Local, public_key_bytes, None, options)
        .await
        .expect("prepare_create_did_csm_with_options failed");

    assert_eq!(signing_req.state.expires_at_unix, Some(far_future_unix));
}

// ---------------------------------------------------------------------------
// update_did empty ops — returns immediately with 0 applied (no network call)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn update_did_with_empty_ops_returns_zero_applied() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");

    let result = ctx
        .sdk
        .update_did(None, create.did.clone(), &create.private_key_bytes, vec![])
        .await
        .expect("update with empty ops should succeed");

    assert_eq!(result.operations_applied, 0);
}
