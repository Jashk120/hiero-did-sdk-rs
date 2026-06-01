use dotenvy::from_filename;
use dotenvy::from_filename_override;
use hiero_did_client::HederaClientConfiguration;
use hiero_did_client::HederaClientService;
use hiero_did_client::HederaNetwork;
use hiero_did_client::NetworkConfig;
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_did_registrar::deactivate::deactivate_did;
use hiero_did_registrar::update::AddService;
use hiero_did_registrar::update::AddVerificationMethod;
use hiero_did_registrar::update::DIDUpdateOperation;
use hiero_did_registrar::update::RemoveService;
use hiero_did_registrar::update::RemoveVerificationMethod;
use hiero_did_registrar::update::VerificationMethodProperty;
use hiero_did_registrar::update::update_did;
use hiero_did_resolver::DidDocumentBuilder;
use hiero_did_resolver::MirrorNodeClient;
use hiero_sdk::Client;
use std::env;

#[cfg(feature = "debug-mirror")]
use serde_json::Value;

fn setup_env() {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");
}

fn get_network() -> Network {
    match env::var("HEDERA_NETWORK")
        .unwrap_or_else(|_| "testnet".to_string())
        .as_str()
    {
        "mainnet" => Network::Mainnet,
        "local" | "local-node" | "localhost" => Network::Testnet, // local uses testnet DID namespace
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

fn setup_mirror() -> MirrorNodeClient {
    setup_env();
    MirrorNodeClient::from_env()
}

#[tokio::test]
async fn test_create_did() {
    let client = setup_client();
    let network = get_network();

    let result = create_did(&client, network, None)
        .await
        .expect("Failed to create DID");

    println!("Created DID: {}", result.did);
    println!("Topic ID: {}", result.did.topic_id);

    assert!(!result.did.topic_id.is_empty());
    assert_eq!(result.public_key_bytes.len(), 32);
    assert_eq!(result.private_key_bytes.len(), 32);
}

#[tokio::test]
async fn test_create_and_resolve_did() {
    let client = setup_client();
    let mirror = setup_mirror();
    let network = get_network();

    let result = create_did(&client, network, None)
        .await
        .expect("Failed to create DID");

    println!("Created DID: {}", result.did);

    let messages = mirror
        .wait_for_mirror(&result.did.topic_id, 30)
        .await
        .expect("Failed to fetch topic messages");

    println!("Got {} messages from topic", messages.len());
    assert!(!messages.is_empty(), "No messages found on topic");

    let resolution = DidDocumentBuilder::from(messages)
        .resolve(&result.did)
        .await
        .expect("Failed to resolve DID");

    println!("Resolved DID document: {}", resolution.did_document.id);

    assert_eq!(resolution.did_document.id, result.did.to_string());
    assert!(!resolution.did_document.verification_method.is_empty());
    assert!(resolution.did_document_metadata.deactivated == Some(false));
}

#[tokio::test]
async fn test_update_did_add_and_remove_operations() {
    let client = setup_client();
    let mirror = setup_mirror();
    let network = get_network();

    let created = create_did(&client, network, None)
        .await
        .expect("Failed to create DID");
    let did = created.did.clone();
    let did_str = did.to_string();

    let vm_id = format!("{}#key-1", did_str);
    let service_id = format!("{}#svc-1", did_str);
    let service_endpoint = "https://example.com/agent".to_string();
    let vm_key = hiero_sdk::PrivateKey::generate_ed25519()
        .public_key()
        .to_bytes_raw();
    let vm_public_key_multibase = hiero_did_core::KeysUtility::from_bytes(vm_key).to_multibase();

    let add_ops = vec![
        DIDUpdateOperation::AddVerificationMethod(AddVerificationMethod {
            id: vm_id.clone(),
            property: VerificationMethodProperty::VerificationMethod,
            controller: None,
            public_key_multibase: Some(vm_public_key_multibase),
        }),
        DIDUpdateOperation::AddService(AddService {
            id: service_id.clone(),
            service_type: "LinkedDomains".to_string(),
            service_endpoint: service_endpoint.clone(),
        }),
    ];

    let add_result = update_did(&client, did.clone(), &created.private_key_bytes, add_ops)
        .await
        .expect("Failed to apply add update operations");
    assert_eq!(add_result.operations_applied, 2);

    let messages = mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Failed to fetch topic messages after add operations");
    let resolution_after_add = DidDocumentBuilder::from(messages)
        .resolve(&did)
        .await
        .expect("Failed to resolve DID after add operations");

    assert!(
        resolution_after_add
            .did_document
            .verification_method
            .iter()
            .any(|vm| vm.id() == vm_id),
        "Added verification method should be present",
    );
    assert!(
        resolution_after_add
            .did_document
            .service
            .unwrap_or_default()
            .iter()
            .any(|svc| svc.id == service_id && svc.service_endpoint == service_endpoint),
        "Added service should be present",
    );

    let remove_ops = vec![
        DIDUpdateOperation::RemoveVerificationMethod(RemoveVerificationMethod {
            id: vm_id.clone(),
        }),
        DIDUpdateOperation::RemoveService(RemoveService {
            id: service_id.clone(),
        }),
    ];

    let remove_result = update_did(&client, did.clone(), &created.private_key_bytes, remove_ops)
        .await
        .expect("Failed to apply remove update operations");
    assert_eq!(remove_result.operations_applied, 2);

    let messages = mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Failed to fetch topic messages after remove operations");
    let resolution_after_remove = DidDocumentBuilder::from(messages)
        .resolve(&did)
        .await
        .expect("Failed to resolve DID after remove operations");

    assert!(
        !resolution_after_remove
            .did_document
            .verification_method
            .iter()
            .any(|vm| vm.id() == vm_id),
        "Removed verification method should not be present",
    );
    assert!(
        !resolution_after_remove
            .did_document
            .service
            .unwrap_or_default()
            .iter()
            .any(|svc| svc.id == service_id),
        "Removed service should not be present",
    );
}

#[tokio::test]
async fn test_deactivate_did() {
    let client = setup_client();
    let mirror = setup_mirror();
    let network = get_network();

    let created = create_did(&client, network, None)
        .await
        .expect("Failed to create DID");
    let did = created.did.clone();

    let deactivated = deactivate_did(&client, did.clone(), &created.private_key_bytes)
        .await
        .expect("Failed to deactivate DID");

    assert_eq!(deactivated.did, did.to_string());
    assert_eq!(deactivated.did_document.id, did.to_string());
    assert_eq!(deactivated.did_document.controller, did.to_string());
    assert!(deactivated.did_document.verification_method.is_empty());

    let messages = mirror
        .wait_for_mirror_stable(&did.topic_id, 1500, 30)
        .await
        .expect("Failed to fetch topic messages after deactivation");
    let resolution = DidDocumentBuilder::from(messages)
        .resolve(&did)
        .await
        .expect("Failed to resolve DID after deactivation");

    assert_eq!(resolution.did_document_metadata.deactivated, Some(true));
}
