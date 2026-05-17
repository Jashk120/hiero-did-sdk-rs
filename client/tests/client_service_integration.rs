use dotenvy::{from_filename, from_filename_override};
use hiero_did_client::{
    HederaClientConfiguration, HederaClientService, HederaNetwork, NetworkConfig,
};
use std::env;

fn env_or_skip(name: &str) -> Option<String> {
    env::var(name).ok()
}

fn test_config_single() -> Option<HederaClientConfiguration> {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");
    let operator_id = env_or_skip("HEDERA_ACCOUNT_ID")?;
    let operator_key = env_or_skip("HEDERA_PRIVATE_KEY")?;
    let network = env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string());

    let network_kind = match network.as_str() {
        "testnet" => HederaNetwork::Testnet,
        "mainnet" => HederaNetwork::Mainnet,
        "previewnet" => HederaNetwork::Previewnet,
        "localhost" => HederaNetwork::LocalNode,
        _ => HederaNetwork::Testnet,
    };

    Some(HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: network_kind,
            operator_id,
            operator_key,
        }],
    })
}

#[tokio::test]
async fn builds_network_client_with_operator_set() {
    let Some(config) = test_config_single() else {
        return;
    };

    let svc = HederaClientService::new(config).expect("service should build");
    let client = svc.get_client(None).expect("client should build");
    assert!(client.get_operator_account_id().is_some());
    assert!(client.get_operator_public_key().is_some());
}

#[test]
fn rejects_empty_networks_config() {
    let cfg = HederaClientConfiguration { networks: vec![] };
    assert!(HederaClientService::new(cfg).is_err());
}

#[test]
fn rejects_duplicate_network_names() {
    let cfg = HederaClientConfiguration {
        networks: vec![
            NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.1001".to_string(),
                operator_key: "302e020100300506032b6570042204200000000000000000000000000000000000000000000000000000000000000000".to_string(),
            },
            NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.1002".to_string(),
                operator_key: "302e020100300506032b6570042204200000000000000000000000000000000000000000000000000000000000000000".to_string(),
            },
        ],
    };
    assert!(HederaClientService::new(cfg).is_err());
}

#[tokio::test]
async fn with_client_runs_operation() {
    let Some(config) = test_config_single() else {
        return;
    };

    let svc = HederaClientService::new(config).expect("service should build");
    let operator = svc
        .with_client(None, |client| async move {
            Ok(client.get_operator_account_id().map(|a| a.to_string()))
        })
        .await
        .expect("operation should succeed");

    assert!(operator.is_some());
}
