use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use dotenvy::from_filename;
use dotenvy::from_filename_override;
use hiero_did_anoncreds::AccumKey;
use hiero_did_anoncreds::AnonCredsCredentialDefinition;
use hiero_did_anoncreds::AnonCredsRevocationRegistryDefinition;
use hiero_did_anoncreds::AnonCredsRevocationStatusList;
use hiero_did_anoncreds::AnonCredsSchema;
use hiero_did_anoncreds::CredentialDefinitionValue;
use hiero_did_anoncreds::HederaAnonCredsRegistry;
use hiero_did_anoncreds::RevocationRegistryDefinitionValue;
use hiero_did_anoncreds::RevocationRegistryPublicKeys;
use hiero_did_client::HederaClientConfiguration;
use hiero_did_client::HederaClientService;
use hiero_did_client::HederaNetwork;
use hiero_did_client::NetworkConfig;
use hiero_did_core::Signer;
use hiero_did_hcs::HederaHcsService;
use hiero_did_hcs::LocalSigner;
use hiero_sdk::PrivateKey;

fn unique_tag(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

struct Ctx {
    network_name: String,
    signer: Arc<dyn Signer>,
    registry: HederaAnonCredsRegistry,
    issuer_did: String,
}

fn setup_ctx() -> Result<Ctx, String> {
    // Force .env.local to override shell env, then fill missing values from .env.
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");

    let operator_id = env::var("HEDERA_ACCOUNT_ID")
        .map_err(|_| "HEDERA_ACCOUNT_ID is required for anoncreds integration tests".to_string())?;
    let operator_key = env::var("HEDERA_PRIVATE_KEY").map_err(|_| {
        "HEDERA_PRIVATE_KEY is required for anoncreds integration tests".to_string()
    })?;
    let network_name = env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string());

    let network = match network_name.as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "testnet" => HederaNetwork::Testnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        other => {
            return Err(format!(
                "Unsupported HEDERA_NETWORK='{other}'. Use one of: testnet, mainnet, previewnet"
            ));
        }
    };

    let client_service = HederaClientService::new(HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network,
            operator_id: operator_id.clone(),
            operator_key: operator_key.clone(),
        }],
    })
    .map_err(|e| format!("Failed to initialize HederaClientService: {e}"))?;

    let hcs_service = HederaHcsService::new(client_service, None);
    let registry = HederaAnonCredsRegistry::new(hcs_service);
    let key = PrivateKey::from_str_der(&operator_key)
        .map_err(|e| format!("Invalid HEDERA_PRIVATE_KEY: {e}"))?;
    let signer: Arc<dyn Signer> = Arc::new(LocalSigner::new(key));
    let did_network_name = match network_name.as_str() {
        "local" | "local-node" | "localhost" => "testnet",
        _ => network_name.as_str(),
    };
    let issuer_did = format!("did:hedera:{did_network_name}:testkey_{}", operator_id);
    println!("ACCOUNT={}", operator_id);

    Ok(Ctx {
        network_name,
        signer,
        registry,
        issuer_did,
    })
}

#[tokio::test]
async fn register_and_get_schema_roundtrip() {
    let ctx = setup_ctx().expect("integration env setup failed");

    let schema = AnonCredsSchema {
        issuer_id: ctx.issuer_did.clone(),
        name: unique_tag("schema"),
        version: "1.0".to_string(),
        attr_names: vec!["name".to_string(), "age".to_string()],
    };

    let schema_id = ctx
        .registry
        .register_schema(
            Some(ctx.network_name.as_str()),
            schema.clone(),
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register schema");

    let resolved = ctx
        .registry
        .get_schema(&schema_id)
        .await
        .expect("get schema");

    assert_eq!(resolved.issuer_id, schema.issuer_id);
    assert_eq!(resolved.name, schema.name);
    assert_eq!(resolved.attr_names, schema.attr_names);
}

#[tokio::test]
async fn register_and_get_credential_definition_roundtrip() {
    let ctx = setup_ctx().expect("integration env setup failed");

    let schema = AnonCredsSchema {
        issuer_id: ctx.issuer_did.clone(),
        name: unique_tag("schema-for-creddef"),
        version: "1.0".to_string(),
        attr_names: vec!["email".to_string()],
    };
    let schema_id = ctx
        .registry
        .register_schema(
            Some(ctx.network_name.as_str()),
            schema,
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register schema");

    let cred_def = AnonCredsCredentialDefinition {
        issuer_id: ctx.issuer_did.clone(),
        schema_id,
        cred_type: "CL".to_string(),
        tag: unique_tag("creddef"),
        value: CredentialDefinitionValue {
            primary: HashMap::new(),
            revocation: None,
        },
    };

    let cred_def_id = ctx
        .registry
        .register_credential_definition(
            Some(ctx.network_name.as_str()),
            cred_def.clone(),
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register cred def");

    let resolved = ctx
        .registry
        .get_credential_definition(&cred_def_id)
        .await
        .expect("get cred def");
    assert_eq!(resolved.issuer_id, cred_def.issuer_id);
    assert_eq!(resolved.tag, cred_def.tag);
    assert_eq!(resolved.cred_type, "CL");
}

#[tokio::test]
async fn register_revocation_registry_and_status_list_roundtrip() {
    let ctx = setup_ctx().expect("integration env setup failed");

    let schema = AnonCredsSchema {
        issuer_id: ctx.issuer_did.clone(),
        name: unique_tag("schema-for-revreg"),
        version: "1.0".to_string(),
        attr_names: vec!["status".to_string()],
    };
    let schema_id = ctx
        .registry
        .register_schema(
            Some(ctx.network_name.as_str()),
            schema,
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register schema");

    let cred_def = AnonCredsCredentialDefinition {
        issuer_id: ctx.issuer_did.clone(),
        schema_id,
        cred_type: "CL".to_string(),
        tag: unique_tag("creddef-for-revreg"),
        value: CredentialDefinitionValue {
            primary: HashMap::new(),
            revocation: None,
        },
    };
    let cred_def_id = ctx
        .registry
        .register_credential_definition(
            Some(ctx.network_name.as_str()),
            cred_def,
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register cred def");

    let rev_reg_def = AnonCredsRevocationRegistryDefinition {
        issuer_id: ctx.issuer_did.clone(),
        revoc_def_type: "CL_ACCUM".to_string(),
        cred_def_id,
        tag: unique_tag("revreg"),
        value: RevocationRegistryDefinitionValue {
            public_keys: RevocationRegistryPublicKeys {
                accum_key: AccumKey { z: "z".to_string() },
            },
            max_cred_num: 8,
            tails_location: "https://example.com/tails".to_string(),
            tails_hash: "hash".to_string(),
        },
    };

    let rev_reg_def_id = ctx
        .registry
        .register_revocation_registry_definition(
            Some(ctx.network_name.as_str()),
            rev_reg_def.clone(),
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register rev reg def");

    let rev_with_meta = ctx
        .registry
        .get_revocation_registry_definition(&rev_reg_def_id)
        .await
        .expect("get rev reg def");
    assert_eq!(rev_with_meta.rev_reg_def.issuer_id, rev_reg_def.issuer_id);
    assert!(!rev_with_meta.hcs_metadata.entries_topic_id.is_empty());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let status_list = AnonCredsRevocationStatusList {
        issuer_id: ctx.issuer_did.clone(),
        rev_reg_def_id: rev_reg_def_id.clone(),
        revocation_list: vec![0, 1, 0, 1, 0, 0, 1, 0],
        current_accumulator: "accum-1".to_string(),
        timestamp: now,
    };

    ctx.registry
        .register_revocation_status_list(
            Some(ctx.network_name.as_str()),
            status_list.clone(),
            Arc::clone(&ctx.signer),
        )
        .await
        .expect("register status list");

    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    let resolved = ctx
        .registry
        .get_revocation_status_list(Some(ctx.network_name.as_str()), &rev_reg_def_id, now + 300)
        .await
        .expect("get status list");
    assert_eq!(resolved.revocation_list, status_list.revocation_list);
    assert_eq!(
        resolved.current_accumulator,
        status_list.current_accumulator
    );
    assert_eq!(resolved.issuer_id, status_list.issuer_id);
}
