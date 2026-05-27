use dotenvy::{from_filename, from_filename_override};
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_did_registrar::deactivate::deactivate_did;
use hiero_did_registrar::update::{
    AddService, AddVerificationMethod, DIDUpdateOperation, RemoveService, RemoveVerificationMethod,
    VerificationMethodProperty, update_did,
};
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient};
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::env;
use std::str::FromStr;

#[cfg(feature = "debug-mirror")]
use serde_json::Value;

fn setup_client() -> Client {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");

    let account_id = env::var("HEDERA_ACCOUNT_ID").expect("HEDERA_ACCOUNT_ID not set");
    let private_key = env::var("HEDERA_PRIVATE_KEY").expect("HEDERA_PRIVATE_KEY not set");

    let client = Client::for_testnet();
    client.set_operator(
        AccountId::from_str(&account_id).expect("Invalid account ID"),
        PrivateKey::from_str_der(&private_key).expect("Invalid private key"),
    );

    client
}

#[tokio::test]
async fn test_create_did() {
    let client = setup_client();

    let result = create_did(&client, Network::Testnet, None)
        .await
        .expect("Failed to create DID");

    println!("Created DID: {}", result.did);
    println!("Topic ID: {}", result.did.topic_id);

    assert!(result.did.to_string().starts_with("did:hedera:testnet:"));
    assert!(!result.did.topic_id.is_empty());
    assert_eq!(result.public_key_bytes.len(), 32);
    assert_eq!(result.private_key_bytes.len(), 32);
}

#[tokio::test]
async fn test_create_and_resolve_did() {
    let client = setup_client();

    let result = create_did(&client, Network::Testnet, None)
        .await
        .expect("Failed to create DID");

    println!("Created DID: {}", result.did);

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    #[cfg(feature = "debug-mirror")]
    {
        let debug_url = format!(
            "https://testnet.mirrornode.hedera.com/api/v1/topics/{}/messages?order=asc&limit=100",
            result.did.topic_id
        );
        println!("Mirror URL: {}", debug_url);
        let debug_response = reqwest::get(&debug_url)
            .await
            .expect("Failed to call mirror debug URL");
        println!("Mirror HTTP status: {}", debug_response.status());
        let debug_text = debug_response
            .text()
            .await
            .expect("Failed to read mirror debug response body");
        println!("Mirror raw JSON bytes: {}", debug_text.len());
        println!(
            "Mirror raw JSON preview: {}",
            &debug_text.chars().take(1200).collect::<String>()
        );

        let debug_json: Value =
            serde_json::from_str(&debug_text).expect("Failed to parse mirror debug JSON");
        let top_keys = debug_json
            .as_object()
            .map(|m| m.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        println!("Mirror top-level keys: {:?}", top_keys);
        let first_msg = debug_json
            .get("messages")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or(Value::Null);
        println!("Mirror first message shape: {}", first_msg);
    }

    let mirror = MirrorNodeClient::for_testnet();
    let messages = mirror
        .get_topic_messages(&result.did.topic_id)
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

    let created = create_did(&client, Network::Testnet, None)
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

    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    let mirror = MirrorNodeClient::for_testnet();
    let messages = mirror
        .get_topic_messages(&did.topic_id)
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

    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    let messages = mirror
        .get_topic_messages(&did.topic_id)
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

    let created = create_did(&client, Network::Testnet, None)
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

    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    let mirror = MirrorNodeClient::for_testnet();
    let messages = mirror
        .get_topic_messages(&did.topic_id)
        .await
        .expect("Failed to fetch topic messages after deactivation");
    let resolution = DidDocumentBuilder::from(messages)
        .resolve(&did)
        .await
        .expect("Failed to resolve DID after deactivation");

    assert_eq!(resolution.did_document_metadata.deactivated, Some(true));
}
