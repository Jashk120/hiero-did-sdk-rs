// use std::collections::HashMap;
// use std::env;
// use std::sync::Arc;
// use std::time::{SystemTime, UNIX_EPOCH};

// use dotenvy::{from_filename, from_filename_override};
// use hiero_did_sdk::{anoncreds, client, core, hcs};
// use hiero_sdk::PrivateKey;

// fn unique_tag(prefix: &str) -> String {
//     let nanos = SystemTime::now()
//         .duration_since(UNIX_EPOCH)
//         .expect("clock")
//         .as_nanos();
//     format!("{prefix}-{nanos}")
// }

// struct Ctx {
//     network_name: String,
//     signer: Arc<dyn core::Signer>,
//     registry: anoncreds::HederaAnonCredsRegistry,
//     issuer_did: String,
// }

// fn setup_ctx() -> Result<Ctx, String> {
//     let _ = from_filename_override(".env.local");
//     let _ = from_filename(".env");

//     let operator_id = env::var("HEDERA_ACCOUNT_ID")
//         .map_err(|_| "HEDERA_ACCOUNT_ID is required for sdk integration tests".to_string())?;
//     let operator_key = env::var("HEDERA_PRIVATE_KEY")
//         .map_err(|_| "HEDERA_PRIVATE_KEY is required for sdk integration tests".to_string())?;
//     let network_name = env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string());

//     let network = match network_name.as_str() {
//         "mainnet" => client::HederaNetwork::Mainnet,
//         "testnet" => client::HederaNetwork::Testnet,
//         "previewnet" => client::HederaNetwork::Previewnet,
//         other => {
//             return Err(format!(
//                 "Unsupported HEDERA_NETWORK='{other}'. Use one of: testnet, mainnet, previewnet"
//             ));
//         }
//     };

//     let client_service = client::HederaClientService::new(client::HederaClientConfiguration {
//         networks: vec![client::NetworkConfig {
//             network,
//             operator_id: operator_id.clone(),
//             operator_key: operator_key.clone(),
//         }],
//     })
//     .map_err(|e| format!("Failed to initialize HederaClientService: {e}"))?;

//     let hcs_service = hcs::HederaHcsService::new(client_service, None);
//     let registry = anoncreds::HederaAnonCredsRegistry::new(hcs_service);
//     let key = PrivateKey::from_str_der(&operator_key)
//         .map_err(|e| format!("Invalid HEDERA_PRIVATE_KEY: {e}"))?;
//     let signer: Arc<dyn core::Signer> = Arc::new(hcs::LocalSigner::new(key));
//     let issuer_did = format!("did:hedera:{network_name}:testkey_{}", operator_id);

//     Ok(Ctx {
//         network_name,
//         signer,
//         registry,
//         issuer_did,
//     })
// }

// #[tokio::test]
// #[ignore = "requires Hedera credentials and network access"]
// async fn sdk_anoncreds_schema_and_cred_def_roundtrip() {
//     let ctx = setup_ctx().expect("integration env setup failed");

//     let schema = anoncreds::AnonCredsSchema {
//         issuer_id: ctx.issuer_did.clone(),
//         name: unique_tag("sdk-schema"),
//         version: "1.0".to_string(),
//         attr_names: vec!["email".to_string()],
//     };

//     let schema_id = ctx
//         .registry
//         .register_schema(
//             Some(ctx.network_name.as_str()),
//             schema,
//             Arc::clone(&ctx.signer),
//         )
//         .await
//         .expect("register schema");

//     let cred_def = anoncreds::AnonCredsCredentialDefinition {
//         issuer_id: ctx.issuer_did.clone(),
//         schema_id,
//         cred_type: "CL".to_string(),
//         tag: unique_tag("sdk-creddef"),
//         value: anoncreds::CredentialDefinitionValue {
//             primary: HashMap::new(),
//             revocation: None,
//         },
//     };

//     let cred_def_id = ctx
//         .registry
//         .register_credential_definition(
//             Some(ctx.network_name.as_str()),
//             cred_def.clone(),
//             Arc::clone(&ctx.signer),
//         )
//         .await
//         .expect("register cred def");

//     let resolved = ctx
//         .registry
//         .get_credential_definition(&cred_def_id)
//         .await
//         .expect("get cred def");
//     assert_eq!(resolved.issuer_id, cred_def.issuer_id);
//     assert_eq!(resolved.tag, cred_def.tag);
// }
