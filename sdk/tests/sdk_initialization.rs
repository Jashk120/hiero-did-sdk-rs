//! Offline unit tests for `HieroDidSdk` initialization.
//!
//! These tests do NOT touch the network. They validate that the SDK constructs
//! correctly from valid configs and fails correctly from invalid ones.

use hiero_did_sdk::HieroDidSdk;
use hiero_did_sdk::client::{
    HederaClientConfiguration,
    HederaNetwork,
    NetworkConfig,
};
use hiero_sdk::PrivateKey;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn dummy_key() -> String {
    PrivateKey::generate_ed25519().to_string_der()
}

fn testnet_config_single() -> HederaClientConfiguration {
    HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: HederaNetwork::Testnet,
            operator_id: "0.0.1234".to_string(),
            operator_key: dummy_key(),
        }],
    }
}

// ---------------------------------------------------------------------------
// Construction — happy paths
// ---------------------------------------------------------------------------

#[test]
fn from_config_succeeds_with_valid_single_network() {
    let sdk = HieroDidSdk::from_config(testnet_config_single(), None);
    assert!(sdk.is_ok(), "expected SDK to build from a valid single-network config");
}

#[tokio::test]
async fn from_config_exposes_client_service_accessor() {
    let sdk = HieroDidSdk::from_config(testnet_config_single(), None).unwrap();
    // Should be able to resolve a client without error
    assert!(sdk.client_service().get_client(None).is_ok());
}

#[test]
fn from_config_exposes_hcs_service_accessor() {
    let sdk = HieroDidSdk::from_config(testnet_config_single(), None).unwrap();
    // Just assert the accessor is accessible (type-level check)
    let _hcs = sdk.hcs_service();
}

#[test]
fn from_config_exposes_anoncreds_accessor() {
    let sdk = HieroDidSdk::from_config(testnet_config_single(), None).unwrap();
    let _ac = sdk.anoncreds();
}

#[test]
fn sdk_is_cloneable() {
    let sdk = HieroDidSdk::from_config(testnet_config_single(), None).unwrap();
    let _cloned = sdk.clone();
}

#[tokio::test]
async fn from_config_with_multi_network_config_succeeds() {
    let cfg = HederaClientConfiguration {
        networks: vec![
            NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.1".to_string(),
                operator_key: dummy_key(),
            },
            NetworkConfig {
                network: HederaNetwork::Mainnet,
                operator_id: "0.0.2".to_string(),
                operator_key: dummy_key(),
            },
        ],
    };
    let sdk = HieroDidSdk::from_config(cfg, None).unwrap();
    // Named lookup — None would fail when there are >1 networks
    assert!(sdk.client_service().get_client(Some("testnet")).is_ok());
    assert!(sdk.client_service().get_client(Some("mainnet")).is_ok());
}

// ---------------------------------------------------------------------------
// Construction — failure paths
// ---------------------------------------------------------------------------

#[test]
fn from_config_fails_with_empty_networks() {
    let cfg = HederaClientConfiguration { networks: vec![] };
    let err = HieroDidSdk::from_config(cfg, None);
    assert!(err.is_err(), "expected error for empty networks config");
}

#[test]
fn from_config_fails_with_duplicate_network_names() {
    let cfg = HederaClientConfiguration {
        networks: vec![
            NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.1".to_string(),
                operator_key: dummy_key(),
            },
            NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.2".to_string(),
                operator_key: dummy_key(),
            },
        ],
    };
    assert!(HieroDidSdk::from_config(cfg, None).is_err());
}

#[test]
fn get_client_fails_without_network_name_when_multi_network() {
    let cfg = HederaClientConfiguration {
        networks: vec![
            NetworkConfig {
                network: HederaNetwork::Testnet,
                operator_id: "0.0.1".to_string(),
                operator_key: dummy_key(),
            },
            NetworkConfig {
                network: HederaNetwork::Mainnet,
                operator_id: "0.0.2".to_string(),
                operator_key: dummy_key(),
            },
        ],
    };
    let sdk = HieroDidSdk::from_config(cfg, None).unwrap();
    // Network name is required when >1 networks are configured
    assert!(sdk.client_service().get_client(None).is_err());
}

#[test]
fn get_client_fails_with_unknown_network_name() {
    let sdk = HieroDidSdk::from_config(testnet_config_single(), None).unwrap();
    assert!(sdk.client_service().get_client(Some("mainnet")).is_err());
}

// ---------------------------------------------------------------------------
// Vault signer construction (no live Vault required — just verifies error path)
// ---------------------------------------------------------------------------

#[cfg(feature = "vault")]
#[tokio::test]
async fn vault_signer_ctor_fails_gracefully_without_live_vault() {
    use hiero_did_sdk::signer::{
        VaultAuth,
        VaultSignerConfig,
    };
    let handle = tokio::task::spawn_blocking(|| {
        let sdk = HieroDidSdk::from_config(
            HederaClientConfiguration {
                networks: vec![NetworkConfig {
                    network: HederaNetwork::Testnet,
                    operator_id: "0.0.1".to_string(),
                    operator_key: PrivateKey::generate_ed25519().to_string_der(),
                }],
            },
            None,
        )
        .unwrap();

        let cfg = VaultSignerConfig::new(
            "http://127.0.0.1:8200",
            VaultAuth::Token("fake-token".into()),
            "missing-key",
        );
        // Must error, not panic
        let result = sdk.new_vault_signer(cfg);
        assert!(result.is_err(), "expected Vault signer to fail without a live Vault server");
    });
    handle.await.unwrap();
}
