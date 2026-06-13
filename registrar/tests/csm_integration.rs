use dotenvy::from_filename;
use dotenvy::from_filename_override;
use hiero_did_client::HederaClientConfiguration;
use hiero_did_client::HederaClientService;
use hiero_did_client::HederaNetwork;
use hiero_did_client::NetworkConfig;
use hiero_did_core::did::Network;
use hiero_did_registrar::AddService;
use hiero_did_registrar::DIDUpdateOperation;
use hiero_did_registrar::prepare_create_did_csm;
use hiero_did_registrar::prepare_deactivate_did_csm;
use hiero_did_registrar::prepare_update_did_csm;
use hiero_did_registrar::submit_create_did_csm;
use hiero_did_registrar::submit_deactivate_did_csm;
use hiero_did_registrar::submit_update_did_csm;
use hiero_did_resolver::DidDocumentBuilder;
use hiero_did_resolver::MirrorNodeClient;
use hiero_did_signer::InternalSigner;
use hiero_sdk::Client;
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use hiero_did_core::keys::KeysUtility;

fn setup_env() {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");
}

fn get_did_network() -> Network {
    match env::var("HEDERA_NETWORK")
        .unwrap_or_else(|_| "testnet".to_string())
        .as_str()
    {
        "mainnet" => Network::Mainnet,
        "local" | "local-node" | "localhost" => Network::Testnet,
        _ => Network::Testnet,
    }
}

fn get_client_network() -> HederaNetwork {
    match env::var("HEDERA_NETWORK")
        .unwrap_or_else(|_| "testnet".to_string())
        .as_str()
    {
        "mainnet" => HederaNetwork::Mainnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        _ => HederaNetwork::Testnet,
    }
}

fn setup_client() -> Client {
    setup_env();

    let operator_id = env::var("HEDERA_ACCOUNT_ID").expect("HEDERA_ACCOUNT_ID not set");
    let operator_key = env::var("HEDERA_PRIVATE_KEY").expect("HEDERA_PRIVATE_KEY not set");
    let service = HederaClientService::new(HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: get_client_network(),
            operator_id,
            operator_key,
        }],
    })
    .expect("Failed to initialize HederaClientService");

    service
        .get_client(None)
        .expect("Failed to build Hedera client")
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
        .to_string()
}

async fn resolve_until<F>(
    mirror: &MirrorNodeClient,
    did: &hiero_did_core::HederaDid,
    mut condition: F,
) -> hiero_did_core::DIDResolution
where
    F: FnMut(&hiero_did_core::DIDResolution) -> bool,
{
    let mut last = None;

    for _ in 0..12 {
        if let Ok(messages) = mirror.get_topic_messages(&did.topic_id).await {
            if let Ok(resolution) = DidDocumentBuilder::from(messages).resolve(did).await {
                if condition(&resolution) {
                    return resolution;
                }
                last = Some(resolution);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    last.expect("DID never resolved to expected state")
}
#[tokio::test]
async fn csm_create_update_deactivate_roundtrip() {
    let client = setup_client();
    let network = get_did_network();
    let signer = InternalSigner::from_bytes(&[13u8; 32]).expect("signer");

    let create_request =
        prepare_create_did_csm(&client, network, signer.verifying_key_bytes(), None)
            .await
            .expect("prepare create csm");
    let did = create_request.state.did.parse().expect("created did");
    let create_signature = signer.sign(&create_request.message_bytes);
    let create_submit = create_request
        .into_submit_request(create_signature)
        .expect("create submit request");
    submit_create_did_csm(&client, create_submit)
        .await
        .expect("submit create csm");

    let mirror = MirrorNodeClient::from_env();
    let create_resolution = resolve_until(&mirror, &did, |_| true).await; // first resolution is fine here

    assert_eq!(create_resolution.did_document.id, did.to_string());
    assert_eq!(
        create_resolution.did_document_metadata.deactivated,
        Some(false)
    );

    let suffix = unique_suffix();
    let update_request = prepare_update_did_csm(
        did.clone(),
        vec![DIDUpdateOperation::AddService(AddService {
            id: format!("{did}#svc-{suffix}"),
            service_type: "LinkedDomains".to_string(),
            service_endpoint: format!("https://example.com/{suffix}"),
        })],
    )
    .await
    .expect("prepare update csm");
    let update_signatures = update_request
        .requests
        .iter()
        .map(|request| signer.sign(&request.message_bytes))
        .collect();
    let update_submit = update_request
        .into_submit_request(update_signatures)
        .expect("update submit request");
    let update_result = submit_update_did_csm(&client, update_submit)
        .await
        .expect("submit update csm");
    assert_eq!(update_result.operations_applied, 1);

    let update_resolution = resolve_until(&mirror, &did, |r| {
        r.did_document.service.as_ref().map_or(false, |svcs| {
            svcs.iter().any(|s| s.id.ends_with(&format!("#svc-{suffix}")))
        })
    }).await;
    assert!(
        update_resolution
            .did_document
            .service
            .unwrap_or_default()
            .iter()
            .any(|service| service.id.ends_with(&format!("#svc-{suffix}")))
    );

    let deactivate_request = prepare_deactivate_did_csm(did.clone()).await.expect("prepare deactivate");
    let deactivate_signature = signer.sign(&deactivate_request.message_bytes);
    let deactivate_submit = deactivate_request
        .into_submit_request(deactivate_signature)
        .expect("deactivate submit request");
    submit_deactivate_did_csm(&client, deactivate_submit)
        .await
        .expect("submit deactivate csm");

    let deactivate_resolution = resolve_until(&mirror, &did, |r| {
        r.did_document_metadata.deactivated == Some(true)
    }).await;
    assert_eq!(
        deactivate_resolution.did_document_metadata.deactivated,
        Some(true)
    );
}

#[tokio::test]

async fn csm_update_multi_operation_batch() {
    let client = setup_client();
    let network = get_did_network();
    let signer = InternalSigner::from_bytes(&[17u8; 32]).expect("signer");

    // Create the DID first.
    let create_request =
        prepare_create_did_csm(&client, network, signer.verifying_key_bytes(), None)
            .await
            .expect("prepare create csm");
    let did = create_request.state.did.parse().expect("created did");
    let create_signature = signer.sign(&create_request.message_bytes);
    let create_submit = create_request
        .into_submit_request(create_signature)
        .expect("create submit request");
    submit_create_did_csm(&client, create_submit)
        .await
        .expect("submit create csm");

    let mirror = MirrorNodeClient::from_env();
    resolve_until(&mirror, &did, |_| true).await;

    // First, add a service we'll remove in the same batch below.
    let suffix = unique_suffix();
    let removable_service_id = format!("{did}#svc-removeme-{suffix}");
    let setup_update = prepare_update_did_csm(
        did.clone(),
        vec![DIDUpdateOperation::AddService(AddService {
            id: removable_service_id.clone(),
            service_type: "LinkedDomains".to_string(),
            service_endpoint: format!("https://example.com/setup-{suffix}"),
        })],
    )
    .await
    .expect("prepare setup update csm");
    let setup_signatures = setup_update
        .requests
        .iter()
        .map(|request| signer.sign(&request.message_bytes))
        .collect();
    let setup_submit = setup_update
        .into_submit_request(setup_signatures)
        .expect("setup submit request");
    submit_update_did_csm(&client, setup_submit)
        .await
        .expect("submit setup update csm");

    resolve_until(&mirror, &did, |_| true).await;

    // Now: one batch with add VM + add service + remove the service above.
    let vm_id = format!("{did}#key-{suffix}");
    let added_service_id = format!("{did}#svc-added-{suffix}");

    let batch_update = prepare_update_did_csm(
        did.clone(),
        vec![
            DIDUpdateOperation::AddVerificationMethod(hiero_did_registrar::AddVerificationMethod {
                id: vm_id.clone(),
                property: hiero_did_registrar::VerificationMethodProperty::VerificationMethod,
                controller: None,
                public_key_multibase: Some(
                    KeysUtility::from_bytes(signer.verifying_key_bytes()).to_multibase(),
                ),
            }),
            DIDUpdateOperation::AddService(AddService {
                id: added_service_id.clone(),
                service_type: "LinkedDomains".to_string(),
                service_endpoint: format!("https://example.com/added-{suffix}"),
            }),
            DIDUpdateOperation::RemoveService(hiero_did_registrar::RemoveService {
                id: removable_service_id.clone(),
            }),
        ],
    )
    .await
    .expect("prepare batch update csm");

    assert_eq!(batch_update.requests.len(), 3);

    let batch_signatures = batch_update
        .requests
        .iter()
        .map(|request| signer.sign(&request.message_bytes))
        .collect();
    let batch_submit = batch_update
        .into_submit_request(batch_signatures)
        .expect("batch submit request");
    let batch_result = submit_update_did_csm(&client, batch_submit)
        .await
        .expect("submit batch update csm");
    assert_eq!(batch_result.operations_applied, 3);

    let resolution = resolve_until(&mirror, &did, |r| {
        r.did_document.verification_method.iter().any(|vm| vm.id() == vm_id)
            && r.did_document.service.as_ref().map_or(false, |svcs| {
                svcs.iter().any(|s| s.id == added_service_id)
            })
    }).await;

    let verification_methods = resolution.did_document.verification_method;
    assert!(verification_methods.iter().any(|vm| vm.id() == vm_id));

    let services = resolution.did_document.service.unwrap_or_default();
    assert!(services.iter().any(|s| s.id == added_service_id));
    assert!(!services.iter().any(|s| s.id == removable_service_id));
}