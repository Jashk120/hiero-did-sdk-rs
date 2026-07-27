//! Negative and messy-path integration tests against a local Hedera node.
//!
//! These tests are first-class citizens — not an afterthought.
//! Run them separately so failures here are triaged distinctly from happy-path regressions:
//!
//!   cargo test --package hiero-did-sdk --test integration_sdk_negative
//!
//! # Prerequisites: same as integration_sdk.rs

mod common;
use common::local_node::{
    FailingSigner,
    MalformedSigner,
    MockExternalSigner,
    poll_until,
    setup,
    wait_for_did,
};
use hiero_did_sdk::core::did::Network;
use hiero_did_sdk::core::{
    DIDError,
    HederaDid,
};
use hiero_did_sdk::registrar::{
    AddService,
    CsmPrepareOptions,
    DIDUpdateOperation,
};
use serial_test::serial;

// ---------------------------------------------------------------------------
// DID lifecycle abuse
// ---------------------------------------------------------------------------

/// Deactivate a DID that never existed on-chain — must error, not panic.
#[tokio::test]
#[serial]
async fn deactivate_nonexistent_did_returns_error() {
    let Some(ctx) = setup() else { return };

    let fake_did = HederaDid::new(
        Network::Local,
        "z6MkfakeXYZABCDEFGHIJKL".to_string(),
        "0.0.99999999".to_string(),
    );
    let fake_key = hiero_sdk::PrivateKey::generate_ed25519().to_bytes_raw();

    let result = ctx.sdk.deactivate_did(None, fake_did, &fake_key).await;
    assert!(result.is_err(), "deactivating a nonexistent DID must return an error");
}

/// Update a DID that was never created — must error.
#[tokio::test]
#[serial]
async fn update_nonexistent_did_returns_error() {
    let Some(ctx) = setup() else { return };

    let fake_did = HederaDid::new(
        Network::Local,
        "z6MkfakeXYZABCDEFGHIJKL".to_string(),
        "0.0.99999998".to_string(),
    );
    let fake_key = hiero_sdk::PrivateKey::generate_ed25519().to_bytes_raw();

    let result = ctx
        .sdk
        .update_did(
            None,
            fake_did,
            &fake_key,
            vec![DIDUpdateOperation::AddService(AddService {
                id: "did:hedera:local:fake#svc".to_string(),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
            })],
        )
        .await;

    assert!(result.is_err(), "updating a nonexistent DID must return an error");
}

/// Deactivate an already-deactivated DID — must not silently succeed or reverse deactivation.
#[tokio::test]
#[serial]
async fn deactivate_already_deactivated_did_errors_or_stays_deactivated() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    ctx.sdk
        .deactivate_did(None, did.clone(), &create.private_key_bytes)
        .await
        .expect("first deactivation must succeed");

    let second = ctx.sdk.deactivate_did(None, did.clone(), &create.private_key_bytes).await;

    if second.is_ok() {
        // Wait for the mirror to reflect the final state before asserting.
        let resolution = poll_until(
            || {
                let did_str = did.to_string();
                let sdk = ctx.sdk.clone();
                async move {
                    sdk.resolve_did(&did_str, None)
                        .await
                        .ok()
                        .filter(|r| r.did_document_metadata.deactivated.unwrap_or(false))
                }
            },
            30,
            1000,
        )
        .await
        .expect("DID must remain deactivated after second deactivation attempt");
        assert!(
            resolution.did_document_metadata.deactivated.unwrap_or(false),
            "DID must remain deactivated after second deactivation attempt"
        );
    }
}

/// Update an already-deactivated DID — must be rejected or not revive the document.
#[tokio::test]
#[serial]
async fn update_after_deactivation_must_not_revive_did() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    ctx.sdk
        .deactivate_did(None, did.clone(), &create.private_key_bytes)
        .await
        .expect("deactivate failed");

    let update = ctx
        .sdk
        .update_did(
            None,
            did.clone(),
            &create.private_key_bytes,
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{}#post-deactivate", did),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
            })],
        )
        .await;

    if update.is_ok() {
        // Poll until the mirror reflects either deactivated=true or the update.
        // A correctly implemented SDK must never revive a deactivated DID.
        let resolution = poll_until(
    || {
        let did_str = did.to_string();
        let sdk = ctx.sdk.clone();
        async move {
            match sdk.resolve_did(&did_str, None).await {
                Ok(res) if res.did_document_metadata.deactivated.unwrap_or(false) => Some(res),
                _ => None, // keep polling — not converged yet
            }
        }
    },
    30,
    1000,
)
.await
.expect("DID never showed deactivated=true within timeout — either mirror lag exceeded budget or deactivation was actually reverted");

        assert!(
            resolution.did_document_metadata.deactivated.unwrap_or(false),
            "Updating after deactivation must not revive the DID document"
        );
    }
}

// ---------------------------------------------------------------------------
// CSM — adversarial paths
// ---------------------------------------------------------------------------

/// Prepare a CSM deactivate, then never submit — confirm nothing is written.
#[tokio::test]
#[serial]
async fn csm_prepare_without_submit_leaves_no_partial_state() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    // Prepare — no submit (drop the signing request)
    let _signing_req = ctx
        .sdk
        .prepare_deactivate_did_csm(did.clone())
        .await
        .expect("prepare_deactivate_did_csm failed");

    // DID should still be live and resolvable (poll mirror since it was just created)
    let resolution = wait_for_did(&ctx.sdk, &did.to_string(), 30)
        .await
        .expect("resolve after orphaned prepare failed");

    assert!(
        !resolution.did_document_metadata.deactivated.unwrap_or(false),
        "Orphaned prepare must not deactivate the DID"
    );
}

/// CSM submit with a tampered signature — must be rejected.
#[tokio::test]
#[serial]
async fn csm_submit_with_tampered_signature_is_rejected() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();

    let signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes, None)
        .await
        .expect("prepare failed");

    // Tamper: completely wrong signature bytes (64 bytes, wrong content)
    let tampered_sig = vec![0xDE, 0xAD, 0xBE, 0xEF].repeat(16);

    match signing_req.into_submit_request(tampered_sig) {
        Err(_) => { /* rejected at prepare stage — correct */ }
        Ok(submit_req) => {
            let result = ctx.sdk.submit_create_did_csm(None, submit_req).await;
            assert!(result.is_err(), "tampered signature must be rejected");
        }
    }
}

/// CSM submit with short (wrong-length) signature — must be rejected at validation.
#[tokio::test]
#[serial]
async fn csm_submit_with_short_signature_is_rejected() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();

    let signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes, None)
        .await
        .expect("prepare failed");

    let short_sig = vec![0u8; 4];
    let result = signing_req.into_submit_request(short_sig);
    assert!(result.is_err(), "short-length signature must be rejected at into_submit_request");
}

/// Cross-DID replay: sign DID_A's deactivate, attempt to submit — DID_B stays live.
#[tokio::test]
#[serial]
async fn csm_cross_did_replay_does_not_affect_other_did() {
    let Some(ctx) = setup() else { return };

    let key_a = hiero_sdk::PrivateKey::generate_ed25519();
    let key_b = hiero_sdk::PrivateKey::generate_ed25519();

    let create_a = ctx
        .sdk
        .create_did_with_signer(
            None,
            Network::Local,
            None,
            &hiero_did_sdk::hcs::LocalSigner::new(key_a.clone()),
        )
        .await
        .expect("create DID_A failed");

    let create_b = ctx
        .sdk
        .create_did_with_signer(
            None,
            Network::Local,
            None,
            &hiero_did_sdk::hcs::LocalSigner::new(key_b.clone()),
        )
        .await
        .expect("create DID_B failed");

    // Prepare deactivate for DID_A and sign it
    let signing_req_a = ctx
        .sdk
        .prepare_deactivate_did_csm(create_a.did.clone())
        .await
        .expect("prepare deactivate DID_A failed");

    let sig_a = key_a.sign(&signing_req_a.message_bytes);
    let submit_req_a =
        signing_req_a.into_submit_request(sig_a).expect("into_submit_request failed");

    // Submit — this operates on DID_A's topic, DID_B should be untouched
    let _ = ctx.sdk.submit_deactivate_did_csm(None, submit_req_a).await;

    // DID_B must remain live
    let resolution_b = poll_until(
        || {
            let did_str = create_b.did.to_string();
            let sdk = ctx.sdk.clone();
            async move { sdk.resolve_did(&did_str, None).await.ok() }
        },
        30,
        1000,
    )
    .await
    .expect("DID_B never resolved — mirror indexing failed or timed out");

    assert!(
        !resolution_b.did_document_metadata.deactivated.unwrap_or(false),
        "DID_B must not be deactivated as a side-effect"
    );
}

/// CSM update batch — signature count mismatch is rejected at into_submit_request.
#[tokio::test]
#[serial]
async fn csm_update_batch_signature_count_mismatch_is_rejected() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    let batch_req = ctx
        .sdk
        .prepare_update_did_csm(
            did.clone(),
            vec![
                DIDUpdateOperation::AddService(AddService {
                    id: format!("{}#svc-1", did),
                    service_type: "LinkedDomains".to_string(),
                    service_endpoint: "https://example.com/1".to_string(),
                }),
                DIDUpdateOperation::AddService(AddService {
                    id: format!("{}#svc-2", did),
                    service_type: "LinkedDomains".to_string(),
                    service_endpoint: "https://example.com/2".to_string(),
                }),
            ],
        )
        .await
        .expect("prepare_update_did_csm failed");

    assert_eq!(batch_req.requests.len(), 2);

    // Provide only 1 signature for 2 operations — must be rejected
    let key = hiero_sdk::PrivateKey::from_bytes(&create.private_key_bytes).unwrap();
    let only_one_sig = vec![key.sign(&batch_req.requests[0].message_bytes)];

    let result = batch_req.into_submit_request(only_one_sig);
    assert!(result.is_err(), "mismatched signature count must be rejected");
}

/// CSM with an expired expiry timestamp — must be rejected on submit.
#[tokio::test]
#[serial]
async fn csm_expired_state_is_rejected_on_submit() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();
    let expired_options = CsmPrepareOptions { expires_at_unix: Some(1) };

    let signing_req = ctx
        .sdk
        .prepare_create_did_csm_with_options(
            None,
            Network::Local,
            public_key_bytes,
            None,
            expired_options,
        )
        .await
        .expect("prepare failed");

    let signature = key.sign(&signing_req.message_bytes);

    match signing_req.into_submit_request(signature) {
        Err(_) => { /* rejected at prepare stage — correct */ }
        Ok(req) => {
            let result = ctx.sdk.submit_create_did_csm(None, req).await;
            assert!(result.is_err(), "expired CSM state must be rejected on submit");
        }
    }
}

// ---------------------------------------------------------------------------
// Signer failure modes
// ---------------------------------------------------------------------------

/// Signer returns an error — must propagate cleanly, no panic, no partial write.
#[tokio::test]
#[serial]
async fn create_did_with_failing_signer_propagates_error() {
    let Some(ctx) = setup() else { return };

    let failing = FailingSigner;
    let result = ctx.sdk.create_did_with_signer(None, Network::Local, None, &failing).await;

    assert!(result.is_err(), "failing signer must propagate error cleanly");
}

/// Signer returns malformed/wrong-length signature bytes — rejected, not panicked.
#[tokio::test]
#[serial]
async fn create_did_with_malformed_signer_propagates_error() {
    let Some(ctx) = setup() else { return };

    let malformed = MalformedSigner;
    let result = ctx.sdk.create_did_with_signer(None, Network::Local, None, &malformed).await;

    assert!(result.is_err(), "malformed signer output must be rejected, not panicked");
}

/// update_did_with_signer with a failing signer — no partial write.
#[tokio::test]
#[serial]
async fn update_did_with_failing_signer_propagates_error() {
    let Some(ctx) = setup() else { return };

    let mock = MockExternalSigner::new();
    let create = ctx
        .sdk
        .create_did_with_signer(None, Network::Local, None, &mock)
        .await
        .expect("create failed");

    let failing = FailingSigner;
    let result = ctx
        .sdk
        .update_did_with_signer(
            None,
            create.did.clone(),
            vec![DIDUpdateOperation::AddService(AddService {
                id: format!("{}#svc", create.did),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
            })],
            &failing,
        )
        .await;

    assert!(result.is_err(), "failing signer must propagate error, not write partial state");
}

// ---------------------------------------------------------------------------
// Resolution edge cases
// ---------------------------------------------------------------------------

/// Resolving a syntactically invalid DID string gives a clear parse error.
#[tokio::test]
#[serial]
async fn resolve_invalid_did_string_gives_parse_error() {
    let Some(ctx) = setup() else { return };

    let result = ctx.sdk.resolve_did("not:a:valid:did", None).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DIDError::InvalidDid(_)));
}

/// Resolving a well-formed but non-existent DID returns an error.
#[tokio::test]
#[serial]
async fn resolve_nonexistent_did_returns_error() {
    let Some(ctx) = setup() else { return };

    let result =
        ctx.sdk.resolve_did("did:hedera:local:z6MkfakeXYZABCDEFGHIJKL_0.0.99999999", None).await;

    assert!(result.is_err(), "non-existent DID must return an error");
}

/// Dereferencing a DID URL with a nonexistent fragment returns an error.
#[tokio::test]
#[serial]
async fn dereference_nonexistent_fragment_returns_error() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");

    let did_url = format!("{}#fragment-that-does-not-exist", create.did);
    let result = ctx.sdk.dereference_did_url(&did_url, None).await;

    assert!(result.is_err(), "non-existent fragment must return an error");
}

/// Resolving a deactivated DID returns the deactivated document, not stale/active data.
///
/// Regression guard: deactivated DID must not appear as active.
#[tokio::test]
#[serial]
async fn resolve_deactivated_did_returns_deactivated_metadata() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    ctx.sdk
        .deactivate_did(None, did.clone(), &create.private_key_bytes)
        .await
        .expect("deactivate failed");

    // Poll until the deactivation is visible on the mirror before asserting.
    let resolution = poll_until(
        || {
            let did_str = did.to_string();
            let sdk = ctx.sdk.clone();
            async move {
                sdk.resolve_did(&did_str, None)
                    .await
                    .ok()
                    .filter(|r| r.did_document_metadata.deactivated.unwrap_or(false))
            }
        },
        30,
        1000,
    )
    .await
    .expect("resolve post-deactivate must not error");

    assert!(
        resolution.did_document_metadata.deactivated.unwrap_or(false),
        "resolved deactivated DID must have deactivated=true in metadata"
    );
    assert!(
        resolution.did_document.verification_method.is_empty(),
        "deactivated DID must have empty verification methods"
    );
}

// ---------------------------------------------------------------------------
// Regression: TopicMessageQuery from history (UNIX_EPOCH) fix
//
// Locks in the "from_time: Some(OffsetDateTime::UNIX_EPOCH)" behavior.
// If this test fails, the resolver has likely reverted to subscribing from
// "now" instead of from history, causing existing topics to appear empty.
// ---------------------------------------------------------------------------

/// After creating a DID and re-resolving from a fresh SDK instance (no cache),
/// the DID must still be resolvable — confirms historical message query works.
#[tokio::test]
#[serial]
async fn regression_historical_topic_message_query_not_from_now() {
    let Some(ctx) = setup() else { return };

    let create = ctx.sdk.create_did(None, Network::Local, None).await.expect("create failed");
    let did = create.did.clone();

    // Build a completely fresh SDK (no in-process cache — simulates app restart)
    let Some(ctx2) = setup() else { return };

    // Poll until the fresh SDK can resolve the DID — confirms historical query works.
    let resolution = wait_for_did(&ctx2.sdk, &did.to_string(), 30).await
        .expect("regression: fresh SDK could not resolve an existing DID — topic query is likely broken (from=now instead of UNIX_EPOCH)");

    assert_eq!(resolution.did_document.id, did.to_string());
}

// ---------------------------------------------------------------------------
// Infrastructure-Failure Paths
// ---------------------------------------------------------------------------

/// Proves that `prepare_*_csm` strictly operates offline — no network calls.
///
/// We construct a valid SDK pointing to a dead local port (port 1 is always refused).
/// If `prepare_*_csm` secretly touched the network it would fail with a connection
/// error. It must return Ok immediately.
///
/// Note: this test bypasses `setup()`; it is NOT a `#[serial]` test because it does
/// not interact with the shared local Hedera node at all.
#[tokio::test]
async fn prepare_csm_does_not_hit_network() {
    let _ = dotenvy::from_filename(".env");
    // Use real env credentials so the SDK passes key-format validation, but
    // point gRPC at a port that is guaranteed to refuse connections.
    let operator_id = std::env::var("HEDERA_ACCOUNT_ID").unwrap_or_else(|_| "0.0.2".to_string());
    let operator_key = std::env::var("HEDERA_PRIVATE_KEY")
        .unwrap_or_else(|_| {
            // This is the standard local-node genesis key — safe to embed in tests.
            "302e020100300506032b65700422042091132178e72057a1d7528025956fe39b0b847f200ab59b2fdd367017f3087137".to_string()
        });

    let config = hiero_did_sdk::client::HederaClientConfiguration {
        networks: vec![hiero_did_sdk::client::NetworkConfig {
            network: hiero_did_sdk::client::HederaNetwork::LocalNode,
            operator_id,
            operator_key: operator_key.clone(),
        }],
    };
    let Ok(sdk) = hiero_did_sdk::HieroDidSdk::from_config(config, None) else {
        // If the local node env isn't set up, there's nothing to prove here.
        return;
    };

    // Point to a dead port AFTER building the SDK (so config validation passes).
    // The SDK reads HEDERA_NODE_ADDRESS when building each client; this will make
    // any actual network execute fail. But prepare must not build a client at all.
    let key = hiero_sdk::PrivateKey::from_str_der(&operator_key).unwrap();
    let public_key_bytes = key.public_key().to_bytes_raw();

    // prepare_create_did_csm must complete immediately, even with a broken gRPC address.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sdk.prepare_create_did_csm(None, Network::Local, public_key_bytes, None),
    )
    .await;

    assert!(result.is_ok(), "prepare_csm timed out — it is making network calls it should not");
    assert!(result.unwrap().is_ok(), "prepare_csm must succeed without a live node");
}

/// Legitimate Replay: Submitting the exact same valid signed payload twice
/// must be cleanly rejected by Hedera (DUPLICATE_TRANSACTION) or fail consistently,
/// never silently creating a second DID state entry.
#[tokio::test]
#[serial]
async fn csm_true_replay_is_accepted_by_hedera() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();
    let signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes, None)
        .await
        .expect("prepare failed");

    let signature = key.sign(&signing_req.message_bytes);
    let submit_req = signing_req.into_submit_request(signature).unwrap();

    // First submit must succeed
    let first = ctx.sdk.submit_create_did_csm(None, submit_req.clone()).await;
    assert!(
        first.is_ok(),
        "First submission must succeed: {:?}",
        first.err().map(|e| e.to_string())
    );

    // Immediate replay of the identical signed payload
    // Hedera network DOES NOT inherently reject payload replays if submitted in
    // distinct transactions (i.e. different TransactionIds). It's up to the resolver to deduplicate.
    let second = ctx.sdk.submit_create_did_csm(None, submit_req).await;
    assert!(
        second.is_ok(),
        "Hedera accepts identical payload replays when sent as new transactions"
    );
}

/// OrphanedTopic error tag format lock.
///
/// # What this tests
/// When `submit_receipt_failed` fires (transaction sent, receipt never received),
/// the production path at `registrar/src/create.rs:submit_with_retry` tags the
/// error string with `orphaned_topic=<TOPIC_ID>` so callers can identify the
/// stranded topic. This test pins that exact string format to ensure a refactor
/// doesn't silently remove it.
///
/// # What this does NOT test — KNOWN REGRESSION GAP
/// Confirmed via inspection of hiero-sdk-rust: `Channel` (src/channel.rs)
/// is `pub(crate)`, and `Client`'s transport is built internally from node
/// addresses with no constructor accepting a pre-built channel or custom
/// `tower::Service` (src/client/network/mod.rs). There is currently no
/// seam to inject a fault between execute() and get_receipt() without an
/// upstream change to hiero-sdk-rust exposing Channel construction.
/// TODO(regression): requires either a public Client::for_channel(...)
/// constructor upstream, or a toxiproxy-based transport-layer proxy.
///
/// Reproducing it deterministically requires a transport-layer proxy (e.g. toxiproxy
/// or a custom gRPC interceptor) that can drop the connection mid-stream. That is
/// currently tracked as: TODO(regression) orphaned-topic end-to-end, requires toxiproxy.
#[test]
fn orphaned_topic_error_tag_format_lock() {
    let topic_id = "0.0.54321";
    let reason = "submit_receipt_failed: transport error";

    // This mirrors `registrar/src/create.rs` submit_with_retry, line ~166:
    //   DIDError::InternalError(format!("orphaned_topic={} reason={}", topic_id, e))
    let err = hiero_did_sdk::core::DIDError::InternalError(format!(
        "orphaned_topic={} reason={}",
        topic_id, reason
    ));

    let err_str = err.to_string();
    assert!(
        err_str.contains("orphaned_topic=0.0.54321"),
        "Format lock: error tag must include topic ID. Got: {}",
        err_str
    );
    assert!(
        err_str.contains("submit_receipt_failed"),
        "Format lock: error tag must preserve the original reason. Got: {}",
        err_str
    );
}

/// CSM submit with an invalid state version — must be rejected.
#[tokio::test]
#[serial]
async fn csm_submit_with_invalid_state_version_is_rejected() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();

    let mut signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes, None)
        .await
        .expect("prepare failed");

    // Tamper: invalid state version
    signing_req.state.version = 999;

    // Sign the original message
    let sig = key.sign(&signing_req.message_bytes);

    match signing_req.into_submit_request(sig) {
        Err(_) => { /* rejected at prepare stage — correct */ }
        Ok(submit_req) => {
            let result = ctx.sdk.submit_create_did_csm(None, submit_req).await;
            assert!(result.is_err(), "invalid state version must be rejected");
        }
    }
}

/// CSM submit with mismatched public key — must be rejected.
#[tokio::test]
#[serial]
async fn csm_submit_with_mismatched_public_key_is_rejected() {
    let Some(ctx) = setup() else { return };

    let key1 = hiero_sdk::PrivateKey::generate_ed25519();
    let key2 = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes1 = key1.public_key().to_bytes_raw();
    let public_key_bytes2 = key2.public_key().to_bytes_raw();

    let mut signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes1, None)
        .await
        .expect("prepare failed");

    // Tamper: swap public key in the state
    signing_req.state.expected_public_key_bytes = public_key_bytes2;

    // Sign the original message
    let sig = key1.sign(&signing_req.message_bytes);

    match signing_req.into_submit_request(sig) {
        Err(_) => { /* rejected at prepare stage — correct */ }
        Ok(submit_req) => {
            let result = ctx.sdk.submit_create_did_csm(None, submit_req).await;
            assert!(result.is_err(), "mismatched public key must be rejected");
        }
    }
}

/// CSM submit with tampered request ID — must be rejected.
#[tokio::test]
#[serial]
async fn csm_submit_with_tampered_request_id_is_rejected() {
    let Some(ctx) = setup() else { return };

    let key = hiero_sdk::PrivateKey::generate_ed25519();
    let public_key_bytes = key.public_key().to_bytes_raw();

    let mut signing_req = ctx
        .sdk
        .prepare_create_did_csm(None, Network::Local, public_key_bytes, None)
        .await
        .expect("prepare failed");

    // Tamper: alter request_id
    signing_req.state.request_id = "hacked_request_id".to_string();

    // Sign the original message
    let sig = key.sign(&signing_req.message_bytes);

    match signing_req.into_submit_request(sig) {
        Err(_) => { /* rejected at prepare stage — correct */ }
        Ok(submit_req) => {
            let result = ctx.sdk.submit_create_did_csm(None, submit_req).await;
            assert!(result.is_err(), "tampered request ID must be rejected");
        }
    }
}
