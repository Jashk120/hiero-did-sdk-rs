#![cfg(feature = "vault")]

use hiero_did_core::error::DIDError;
use hiero_did_signer::{VaultAuth, VaultSigner, VaultSignerConfig};
use hiero_did_core::signer::Signer;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that a VaultSigner gracefully handles an unreachable endpoint instead of panicking.
#[test]
fn vault_signer_connection_refused_on_init() {
    let config = VaultSignerConfig::new(
        "http://127.0.0.1:65535",
        VaultAuth::Token("fake-token".to_string()),
        "test-key",
    );

    let result = VaultSigner::new(config);

    let err = match result {
        Ok(_) => panic!("VaultSigner initialization must fail when endpoint is unreachable"),
        Err(e) => e,
    };

    assert!(
        matches!(err, DIDError::InternalError(_)),
        "Expected InternalError wrapping the HTTP error, got {:?}",
        err
    );

    let err_msg = err.to_string();
    assert!(
        err_msg.contains("error sending request"),
        "Expected connection error message, got: {}",
        err_msg
    );
}

#[tokio::test]
async fn vault_signer_handles_403_forbidden() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/test-key"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&mock_server)
        .await;

    let config = VaultSignerConfig::new(
        &mock_server.uri(),
        VaultAuth::Token("fake-token".to_string()),
        "test-key",
    );

    let result = tokio::task::spawn_blocking(move || VaultSigner::new(config))
        .await
        .expect("spawn_blocking failed");

    let err = match result {
        Ok(_) => panic!("VaultSigner initialization must fail on 403"),
        Err(e) => e,
    };

    assert!(
        matches!(err, DIDError::InternalError(_)),
        "Expected InternalError wrapping the HTTP error, got {:?}",
        err
    );
    let err_msg = err.to_string();
    assert!(err_msg.contains("error decoding response body"), "Got: {}", err_msg);
}

#[tokio::test]
async fn vault_signer_handles_500_internal_error() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/test-key"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let config = VaultSignerConfig::new(
        &mock_server.uri(),
        VaultAuth::Token("fake-token".to_string()),
        "test-key",
    );

    let result = tokio::task::spawn_blocking(move || VaultSigner::new(config))
        .await
        .expect("spawn_blocking failed");

    let err = match result {
        Ok(_) => panic!("VaultSigner initialization must fail on 500"),
        Err(e) => e,
    };

    assert!(
        matches!(err, DIDError::InternalError(_)),
        "Expected InternalError wrapping the HTTP error, got {:?}",
        err
    );
    let err_msg = err.to_string();
    assert!(err_msg.contains("error decoding response body"), "Got: {}", err_msg);
}

#[tokio::test]
async fn vault_signer_handles_corrupted_json() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "keys": {
                    "1": { "public_key": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" }
                }
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/transit/sign/test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not valid json"))
        .mount(&mock_server)
        .await;

    let config = VaultSignerConfig::new(
        &mock_server.uri(),
        VaultAuth::Token("fake-token".to_string()),
        "test-key",
    );

    let result = tokio::task::spawn_blocking(move || {
        let signer = VaultSigner::new(config).expect("init should succeed");
        signer.sign_bytes(b"hello world")
    })
    .await
    .expect("spawn_blocking failed");
    
    let err = match result {
        Ok(_) => panic!("VaultSigner sign_bytes must fail on corrupted JSON"),
        Err(e) => e,
    };

    assert!(
        matches!(err, DIDError::InternalError(_)),
        "Expected InternalError wrapping the JSON parse error, got {:?}",
        err
    );
    let err_msg = err.to_string();
    assert!(err_msg.contains("error decoding response body") || err_msg.contains("expected ident"), "Got: {}", err_msg);
}
