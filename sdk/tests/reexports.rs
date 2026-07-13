use hiero_did_sdk::{
    HieroDidSdk,
    anoncreds,
    client,
    core,
    hcs,
    messages,
    method,
    registrar,
    resolver,
    signer,
};

#[test]
fn sdk_reexports_are_accessible() {
    let _did = core::HederaDid::new(
        core::did::Network::Testnet,
        "base58key".to_string(),
        "0.0.123".to_string(),
    );

    let _schema = anoncreds::AnonCredsSchema {
        issuer_id: "did:hedera:testnet:key_0.0.123".to_string(),
        name: "schema".to_string(),
        version: "1.0".to_string(),
        attr_names: vec!["name".to_string()],
    };

    let _network_name = client::service::NetworkName::new("testnet");
    let _msg: Option<messages::HcsMessage> = None;
    let _topic_info: Option<hcs::TopicInfo> = None;
    let _mirror_client = resolver::MirrorNodeClient::for_testnet();
    let _create_result: Option<registrar::CreateDIDResult> = None;
    let _did_parse = method::parse_did("did:hedera:testnet:abc_0.0.1");
    let _core_signer_trait: Option<&dyn core::Signer> = None;
    let _crate_marker = std::any::type_name::<signer::InternalSigner>();
}

#[tokio::test]
async fn test_sdk_handler_instantiation_from_config() {
    let config = client::HederaClientConfiguration {
        networks: vec![client::NetworkConfig {
            network: client::HederaNetwork::Testnet,
            operator_id: "0.0.123".to_string(),
            operator_key: hiero_sdk::PrivateKey::generate_ed25519().to_string_der(),
        }],
    };

    let sdk =
        HieroDidSdk::from_config(config, None).expect("failed to instantiate SDK from config");
    assert!(sdk.client_service().get_client(None).is_ok());

    // Verify resolving a DID doesn't panic and handles input correctly
    // (It might return a resolution error for fake DIDs but the parsing path is hit)
    let result = sdk
        .resolve_did(
            "did:hedera:testnet:z6MkpTHR8VNsBxRcmSt62E7SdTcv9B8ndv5Tzz6A4j5b2j3m_0.0.123",
            None,
        )
        .await;
    assert!(result.is_err()); // Expected since mirror nodes won't have it
}

#[cfg(feature = "vault")]
#[tokio::test]
async fn test_new_vault_signer_instantiates_with_config() {
    let handle = tokio::task::spawn_blocking(|| {
        let config = client::HederaClientConfiguration {
            networks: vec![client::NetworkConfig {
                network: client::HederaNetwork::Testnet,
                operator_id: "0.0.123".to_string(),
                operator_key: hiero_sdk::PrivateKey::generate_ed25519().to_string_der(),
            }],
        };

        let sdk =
            HieroDidSdk::from_config(config, None).expect("failed to instantiate SDK from config");
        let vault_config = signer::VaultSignerConfig::new(
            "http://localhost:8200",
            signer::VaultAuth::Token("token".into()),
            "key",
        );
        // Since Vault is not running, calling VaultSigner::new will try log in and fail,
        // but this proves the function is compiled and called.
        let signer = sdk.new_vault_signer(vault_config);
        assert!(signer.is_err());
    });
    handle.await.unwrap();
}
