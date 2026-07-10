//! Offline unit tests for DID resolution and URL dereference via `HieroDidSdk`.
//!
//! Uses a `MockTopicReader` that implements the `TopicReader` trait and returns
//! pre-programmed responses — no network required. This proves the SDK's parsing
//! and resolution pipeline work correctly in isolation.

use async_trait::async_trait;
use hiero_did_sdk::{
    HieroDidSdk,
    client::{HederaClientConfiguration, HederaNetwork, NetworkConfig},
    core::DIDError,
    resolver::TopicReader,
};
use hiero_sdk::PrivateKey;

// ---------------------------------------------------------------------------
// MockTopicReader variants
// ---------------------------------------------------------------------------

/// Returns an empty message list — simulates a DID topic with no messages.
struct EmptyTopicReader;

#[async_trait]
impl TopicReader for EmptyTopicReader {
    async fn get_topic_messages(&self, _topic_id: &str) -> Result<Vec<String>, DIDError> {
        Ok(vec![])
    }
}

/// Always returns an error — simulates a mirror node being unreachable.
struct ErrorTopicReader;

#[async_trait]
impl TopicReader for ErrorTopicReader {
    async fn get_topic_messages(&self, _topic_id: &str) -> Result<Vec<String>, DIDError> {
        Err(DIDError::InternalError("mock mirror node unreachable".into()))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_sdk() -> HieroDidSdk {
    HieroDidSdk::from_config(
        HederaClientConfiguration {
            networks: vec![NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.1234".to_string(),
                operator_key: PrivateKey::generate_ed25519().to_string_der(),
            }],
        },
        None,
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// resolve_did — input validation (no reader needed, fails at parse)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_did_rejects_completely_invalid_string() {
    let sdk = make_sdk();
    let err = sdk.resolve_did("not-a-did-at-all", None).await;
    assert!(err.is_err(), "must reject non-DID string");
    assert!(matches!(err.unwrap_err(), DIDError::InvalidDid(_)));
}

#[tokio::test]
async fn resolve_did_rejects_did_without_hedera_method() {
    let sdk = make_sdk();
    let err = sdk.resolve_did("did:example:abc123", None).await;
    assert!(err.is_err(), "must reject non-hedera DID method");
}

#[tokio::test]
async fn resolve_did_rejects_empty_string() {
    let sdk = make_sdk();
    let err = sdk.resolve_did("", None).await;
    assert!(err.is_err(), "must reject empty DID string");
}

// ---------------------------------------------------------------------------
// resolve_did — mock reader paths
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_did_with_empty_topic_returns_error() {
    let sdk = make_sdk();
    // A syntactically valid DID with an empty topic — resolver should surface
    // a NotFound or resolution error, not panic.
    let reader = EmptyTopicReader;
    let result = sdk
        .resolve_did(
            "did:hedera:testnet:z6MkpTHR8VNsBxRcmSt62E7SdTcv9B8ndv5Tzz6A4j5b2j3m_0.0.12345",
            Some(&reader),
        )
        .await;
    assert!(result.is_err(), "no messages on topic should produce a resolution error");
}

#[tokio::test]
async fn resolve_did_surfaces_reader_transport_error() {
    let sdk = make_sdk();
    let reader = ErrorTopicReader;
    let result = sdk
        .resolve_did(
            "did:hedera:testnet:z6MkpTHR8VNsBxRcmSt62E7SdTcv9B8ndv5Tzz6A4j5b2j3m_0.0.12345",
            Some(&reader),
        )
        .await;
    assert!(result.is_err(), "reader transport error must propagate");
}

// ---------------------------------------------------------------------------
// dereference_did_url — input validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dereference_did_url_rejects_invalid_url() {
    let sdk = make_sdk();
    let err = sdk.dereference_did_url("not-a-did-url", None).await;
    assert!(err.is_err(), "must reject non-DID URL string");
}

#[tokio::test]
async fn dereference_did_url_rejects_empty_string() {
    let sdk = make_sdk();
    let err = sdk.dereference_did_url("", None).await;
    assert!(err.is_err(), "must reject empty DID URL string");
}

#[tokio::test]
async fn dereference_did_url_propagates_reader_error() {
    let sdk = make_sdk();
    let reader = ErrorTopicReader;
    let result = sdk
        .dereference_did_url(
            "did:hedera:testnet:z6MkpTHR8VNsBxRcmSt62E7SdTcv9B8ndv5Tzz6A4j5b2j3m_0.0.12345#did-root-key",
            Some(&reader),
        )
        .await;
    assert!(result.is_err(), "reader transport error must propagate through dereference");
}

#[tokio::test]
async fn dereference_did_url_with_empty_topic_returns_error() {
    let sdk = make_sdk();
    let reader = EmptyTopicReader;
    let result = sdk
        .dereference_did_url(
            "did:hedera:testnet:z6MkpTHR8VNsBxRcmSt62E7SdTcv9B8ndv5Tzz6A4j5b2j3m_0.0.12345#did-root-key",
            Some(&reader),
        )
        .await;
    assert!(result.is_err(), "no messages on topic should fail dereference");
}
