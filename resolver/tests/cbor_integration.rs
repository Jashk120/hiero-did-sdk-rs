use std::env;

use dotenvy::{from_filename, from_filename_override};
use hiero_did_client::{
    HederaClientConfiguration, HederaClientService, HederaNetwork, NetworkConfig,
};
use hiero_did_core::{Accept, DIDDocument, did::Network};
use hiero_did_registrar::create::create_did;
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient, representation::represent};

fn setup_env() {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");
}

fn get_network() -> Network {
    match env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string()).as_str() {
        "mainnet" => Network::Mainnet,
        "local" | "local-node" | "localhost" => Network::Testnet,
        _ => Network::Testnet,
    }
}

fn get_client_network() -> HederaNetwork {
    match env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string()).as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        _ => HederaNetwork::Testnet,
    }
}

fn setup_client() -> hiero_sdk::Client {
    setup_env();
    let operator_id = env::var("HEDERA_ACCOUNT_ID").expect("HEDERA_ACCOUNT_ID not set");
    let operator_key = env::var("HEDERA_PRIVATE_KEY").expect("HEDERA_PRIVATE_KEY not set");
    let service = HederaClientService::new(HederaClientConfiguration {
        networks: vec![NetworkConfig { network: get_client_network(), operator_id, operator_key }],
    })
    .expect("Failed to initialize HederaClientService");
    service.get_client(None).expect("Failed to build Hedera client")
}

fn setup_mirror() -> MirrorNodeClient {
    setup_env();
    MirrorNodeClient::from_env()
}

/// Create a real DID and resolve it — returns the resolved document and raw messages.
async fn create_and_resolve() -> (hiero_did_core::HederaDid, DIDDocument) {
    let client = setup_client();
    let mirror = setup_mirror();
    let network = get_network();

    let created = create_did(&client, network, None).await.expect("Failed to create DID");

    let messages = mirror
        .wait_for_mirror(&created.did.topic_id, 30)
        .await
        .expect("Timed out waiting for mirror");

    let resolution = DidDocumentBuilder::from(messages)
        .resolve(&created.did)
        .await
        .expect("Failed to resolve DID");

    (created.did, resolution.did_document)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Encode a live-resolved DID document to CBOR and decode it back — all fields
/// must survive the round-trip intact.
#[tokio::test]
async fn cbor_round_trip_live_document() {
    let (_, doc) = create_and_resolve().await;

    let mut encoded = Vec::new();
    ciborium::ser::into_writer(&doc, &mut encoded).expect("CBOR encode failed");
    assert!(!encoded.is_empty(), "encoded bytes must not be empty");

    let decoded: DIDDocument =
        ciborium::de::from_reader(encoded.as_slice()).expect("CBOR decode failed");

    assert_eq!(decoded.id, doc.id);
    assert_eq!(decoded.controller, doc.controller);
    assert_eq!(decoded.context, doc.context);
    assert_eq!(decoded.verification_method, doc.verification_method);
    assert_eq!(decoded.service, doc.service);
    assert_eq!(decoded.authentication, doc.authentication);
    assert_eq!(decoded.assertion_method, doc.assertion_method);
}

/// `represent()` with `Accept::DidCbor` on a live resolution must produce
/// non-empty bytes that decode back to the same document.
#[tokio::test]
async fn represent_cbor_live_resolution() {
    let client = setup_client();
    let mirror = setup_mirror();
    let network = get_network();

    let created = create_did(&client, network, None).await.expect("Failed to create DID");
    let messages = mirror
        .wait_for_mirror(&created.did.topic_id, 30)
        .await
        .expect("Timed out waiting for mirror");

    let resolution =
        DidDocumentBuilder::from(messages).resolve(&created.did).await.expect("Failed to resolve");

    let represented = represent(&resolution, Accept::DidCbor).expect("represent failed");

    let bytes = match represented {
        hiero_did_core::RepresentedDocument::Cbor(b) => b,
        _ => panic!("expected Cbor variant"),
    };

    assert!(!bytes.is_empty());

    let decoded: DIDDocument = ciborium::de::from_reader(bytes.as_slice())
        .expect("CBOR decode of represent output failed");

    assert_eq!(decoded.id, resolution.did_document.id);
    assert_eq!(decoded.verification_method, resolution.did_document.verification_method);
}

/// CBOR encoding of a live document must be smaller than its JSON equivalent.
#[tokio::test]
async fn cbor_smaller_than_json_live() {
    let (_, doc) = create_and_resolve().await;

    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&doc, &mut cbor_bytes).expect("CBOR encode failed");

    let json_bytes = serde_json::to_vec(&doc).expect("JSON encode failed");

    assert!(
        cbor_bytes.len() < json_bytes.len(),
        "CBOR ({} bytes) should be smaller than JSON ({} bytes)",
        cbor_bytes.len(),
        json_bytes.len()
    );
}

/// Encoding the same live document twice must yield identical bytes.
#[tokio::test]
async fn cbor_deterministic_live() {
    let (_, doc) = create_and_resolve().await;

    let mut first = Vec::new();
    ciborium::ser::into_writer(&doc, &mut first).expect("first encode failed");

    let mut second = Vec::new();
    ciborium::ser::into_writer(&doc, &mut second).expect("second encode failed");

    assert_eq!(first, second, "CBOR encoding must be deterministic across calls");
}
