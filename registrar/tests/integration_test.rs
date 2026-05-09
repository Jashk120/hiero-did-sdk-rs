use dotenvy::dotenv;
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient};
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::env;
use std::str::FromStr;

#[cfg(feature = "debug-mirror")]
use serde_json::Value;

fn setup_client() -> Client {
    dotenv().ok();

    let account_id = env::var("HEDERA_ACCOUNT_ID")
        .expect("HEDERA_ACCOUNT_ID not set");
    let private_key = env::var("HEDERA_PRIVATE_KEY")
        .expect("HEDERA_PRIVATE_KEY not set");

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
        println!("Mirror raw JSON preview: {}", &debug_text.chars().take(1200).collect::<String>());

        let debug_json: Value = serde_json::from_str(&debug_text)
            .expect("Failed to parse mirror debug JSON");
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