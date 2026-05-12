use hiero_did_sdk::{anoncreds, client, core, hcs, messages, method, registrar, resolver, signer};

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
