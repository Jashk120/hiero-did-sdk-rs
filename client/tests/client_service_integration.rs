use dotenvy::from_filename;
use dotenvy::from_filename_override;
use hiero_did_client::HederaClientConfiguration;
use hiero_did_client::HederaClientService;
use hiero_did_client::HederaNetwork;
use hiero_did_client::NetworkConfig;
use hiero_sdk::PrivateKey;
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
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
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

#[test]
fn local_node_network_uses_local_defaults_and_aliases() {
    let operator_key = PrivateKey::generate_ed25519().to_string_der();
    let expected_node =
        env::var("HEDERA_NODE_ADDRESS").unwrap_or_else(|_| "127.0.0.1:35211".to_string());
    let expected_mirror =
        env::var("HEDERA_MIRROR_NODE_ADDRESS").unwrap_or_else(|_| "127.0.0.1:38081".to_string());
    let svc = HederaClientService::new(HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: HederaNetwork::LocalNode,
            operator_id: "0.0.2".to_string(),
            operator_key,
        }],
    })
    .expect("service should build");

    for alias in ["local-node", "localhost", "local"] {
        let client = svc.get_client(Some(alias)).expect("client should build");
        assert!(client.network().contains_key(&expected_node));
        assert_eq!(client.mirror_network(), vec![expected_mirror.clone()]);
        assert_eq!(
            client.get_operator_account_id().map(|id| id.to_string()),
            Some("0.0.2".to_string())
        );
    }
}