use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use dotenvy::{from_filename, from_filename_override};
use hiero_did_client::{
    HederaClientConfiguration, HederaClientService, HederaCustomNetwork, HederaNetwork,
    NetworkConfig,
};
use hiero_did_core::Signer;
use hiero_did_hcs::{
    CreateTopicProps, GetTopicMessagesProps, HcsCacheService, HcsFileService, HcsMessage, HcsTopic,
    HederaHcsService, LocalSigner, ResolveFileProps, SubmitFileProps, UpdateTopicProps,
};
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

const HCS1_MEMO_PATTERN: &str = ":zstd:base64";
const BASE64_JSON_CONTENT_PREFIX: &str = "data:application/json;base64,";
const WAIT_SECS: u64 = 12;

#[derive(serde::Serialize)]
struct ChunkMessage {
    o: usize,
    c: String,
}

struct EnvCtx {
    client: Client,
    network_name: String,
    operator_id: String,
    operator_key: String,
}

fn unique_tag(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

fn setup_ctx() -> Option<EnvCtx> {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");
    let operator_id = env::var("HEDERA_ACCOUNT_ID").ok()?;
    let operator_key = env::var("HEDERA_PRIVATE_KEY").ok()?;
    let network_name = env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string());

    let network = match network_name.as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "testnet" => HederaNetwork::Testnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        _ => return None,
    };

    let config = HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network,
            operator_id: operator_id.clone(),
            operator_key: operator_key.clone(),
        }],
    };

    let client_service = HederaClientService::new(config).ok()?;
    let client = client_service.get_client(None).ok()?;

    Some(EnvCtx {
        client,
        network_name,
        operator_id,
        operator_key,
    })
}

async fn mirror_wait() {
    tokio::time::sleep(tokio::time::Duration::from_secs(WAIT_SECS)).await;
}

async fn resolve_file_with_retries(
    svc: &HcsFileService<'_>,
    topic_id: String,
    attempts: usize,
) -> Result<Vec<u8>, hiero_did_core::DIDError> {
    let mut last_err = None;
    for _ in 0..attempts {
        match svc
            .resolve_file(&ResolveFileProps {
                topic_id: topic_id.clone(),
            })
            .await
        {
            Ok(payload) => return Ok(payload),
            Err(e) => {
                last_err = Some(e);
                mirror_wait().await;
            }
        }
    }
    Err(last_err.expect("at least one attempt"))
}

fn chunk_payload(payload: &[u8]) -> Vec<String> {
    let compressed = zstd::encode_all(payload, 0).expect("zstd compress");
    let b64 = BASE64.encode(&compressed);
    let content = format!("{BASE64_JSON_CONTENT_PREFIX}{b64}");
    content
        .as_bytes()
        .chunks(960)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect()
}

fn service_config_single(ctx: &EnvCtx) -> HederaClientConfiguration {
    let network = match ctx.network_name.as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        _ => HederaNetwork::Testnet,
    };
    HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network,
            operator_id: ctx.operator_id.clone(),
            operator_key: ctx.operator_key.clone(),
        }],
    }
}

fn service_config_dual(ctx: &EnvCtx) -> HederaClientConfiguration {
    let primary = match ctx.network_name.as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        _ => HederaNetwork::Testnet,
    };
    let nodes: HashMap<String, AccountId> = ctx.client.network();
    let mirrors = ctx.client.mirror_network();
    let custom = HederaNetwork::Custom(HederaCustomNetwork {
        name: "customnet".to_string(),
        nodes,
        mirror_nodes: Some(mirrors),
    });
    HederaClientConfiguration {
        networks: vec![
            NetworkConfig {
                network: primary,
                operator_id: ctx.operator_id.clone(),
                operator_key: ctx.operator_key.clone(),
            },
            NetworkConfig {
                network: custom,
                operator_id: ctx.operator_id.clone(),
                operator_key: ctx.operator_key.clone(),
            },
        ],
    }
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_topic_create_returns_topic_id() {
    let Some(ctx) = setup_ctx() else { return };
    let topic_id = HcsTopic::create(&ctx.client).await.expect("create");
    assert!(topic_id.to_string().starts_with("0.0."));
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_topic_create_with_memo_visible_via_get_info() {
    let Some(ctx) = setup_ctx() else { return };
    let memo = unique_tag("memo");
    let topic_id = HcsTopic::create_with_memo(&ctx.client, &memo)
        .await
        .expect("create_with_memo");
    mirror_wait().await;
    let info = HcsTopic::get_info(&ctx.client, topic_id)
        .await
        .expect("get_info");
    assert_eq!(info.topic_memo, memo);
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_topic_create_with_props_sets_submit_key() {
    let Some(ctx) = setup_ctx() else { return };
    let operator_key = PrivateKey::from_str_der(&ctx.operator_key).expect("key");
    let submit_signer: Arc<dyn Signer> = Arc::new(LocalSigner::new(operator_key));
    let topic_id = HcsTopic::create_with_props(
        &ctx.client,
        CreateTopicProps {
            submit_key_signer: Some(submit_signer),
            ..Default::default()
        },
    )
    .await
    .expect("create_with_props");
    mirror_wait().await;
    let info = HcsTopic::get_info(&ctx.client, topic_id)
        .await
        .expect("get_info");
    assert!(info.submit_key.is_some());
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_topic_update_memo_and_delete() {
    let Some(ctx) = setup_ctx() else { return };
    let admin_signer: Arc<dyn Signer> = Arc::new(LocalSigner::new(
        PrivateKey::from_str_der(&ctx.operator_key).expect("key"),
    ));
    let topic_id = HcsTopic::create_with_props(
        &ctx.client,
        CreateTopicProps {
            admin_key_signer: Some(Arc::clone(&admin_signer)),
            ..Default::default()
        },
    )
    .await
    .expect("create");

    let new_memo = unique_tag("upd");
    HcsTopic::update(
        &ctx.client,
        UpdateTopicProps {
            topic_id,
            topic_memo: Some(new_memo.clone()),
            auto_renew_period_seconds: None,
            expiration_time: None,
            admin_key_signer: Arc::clone(&admin_signer),
            wait_for_visibility: true,
            wait_timeout_ms: Some(30_000),
        },
    )
    .await
    .expect("update");
    let info = HcsTopic::get_info(&ctx.client, topic_id)
        .await
        .expect("get_info");
    assert_eq!(info.topic_memo, new_memo);

    HcsTopic::delete(&ctx.client, topic_id)
        .await
        .expect("delete");
    mirror_wait().await;
    assert!(HcsTopic::get_info(&ctx.client, topic_id).await.is_err());
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_topic_submit_returns_sequence_number() {
    let Some(ctx) = setup_ctx() else { return };
    let topic_id = HcsTopic::create(&ctx.client).await.expect("create");
    let res = HcsTopic::submit(&ctx.client, topic_id, b"hello".to_vec())
        .await
        .expect("submit");
    assert!(res.sequence_number >= 1);
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_message_submit_and_get_topic_messages() {
    let Some(ctx) = setup_ctx() else { return };
    let topic_id = HcsTopic::create(&ctx.client).await.expect("create");
    HcsMessage::submit(&ctx.client, topic_id, b"first".to_vec(), None)
        .await
        .expect("submit first");
    HcsMessage::submit(&ctx.client, topic_id, b"second".to_vec(), None)
        .await
        .expect("submit second");
    mirror_wait().await;

    let msgs = HcsMessage::get_topic_messages(
        &ctx.client,
        GetTopicMessagesProps {
            topic_id,
            from_time: Some(OffsetDateTime::UNIX_EPOCH),
            to_time: None,
            limit: Some(10),
            max_idle_seconds: Some(12),
        },
    )
    .await
    .expect("get messages");
    assert!(msgs.len() >= 2);
    let mut ordered = msgs;
    ordered.sort_by_key(|m| m.sequence_number);
    let tail = &ordered[ordered.len() - 2..];
    assert_eq!(tail[0].contents, b"first");
    assert_eq!(tail[1].contents, b"second");
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_message_submit_with_submit_key_signer() {
    let Some(ctx) = setup_ctx() else { return };
    let submit_signer: Arc<dyn Signer> = Arc::new(LocalSigner::new(PrivateKey::generate_ed25519()));
    let topic_id = HcsTopic::create_with_props(
        &ctx.client,
        CreateTopicProps {
            submit_key_signer: Some(Arc::clone(&submit_signer)),
            ..Default::default()
        },
    )
    .await
    .expect("create access-controlled topic");

    assert!(
        HcsMessage::submit(&ctx.client, topic_id, b"unsigned".to_vec(), None)
            .await
            .is_err()
    );

    assert!(
        HcsMessage::submit(
            &ctx.client,
            topic_id,
            b"signed".to_vec(),
            Some(Arc::clone(&submit_signer)),
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_file_service_submit_and_resolve() {
    let Some(ctx) = setup_ctx() else { return };
    let submit_signer: Arc<dyn Signer> = Arc::new(LocalSigner::new(
        PrivateKey::from_str_der(&ctx.operator_key).expect("key"),
    ));
    let payload = br#"{"hello":"world"}"#.to_vec();
    let svc = HcsFileService::new(&ctx.client, ctx.network_name.clone(), None);
    let topic_id = svc
        .submit_file(SubmitFileProps {
            payload: payload.clone(),
            submit_key_signer: Arc::clone(&submit_signer),
            wait_for_visibility: false,
            wait_timeout_ms: None,
        })
        .await
        .expect("submit_file");
    mirror_wait().await;
    let resolved = resolve_file_with_retries(&svc, topic_id, 10)
        .await
        .expect("resolve_file");
    assert_eq!(resolved, payload);
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_file_service_resolve_uses_cache() {
    let Some(ctx) = setup_ctx() else { return };
    let cache = HcsCacheService::with_defaults();
    let svc = HcsFileService::new(&ctx.client, ctx.network_name.clone(), Some(cache.clone()));
    let fake_topic = "0.0.999999999999".to_string();
    let payload = b"cached".to_vec();
    cache
        .set_topic_file(&ctx.network_name, &fake_topic, &payload)
        .await;
    let resolved = svc
        .resolve_file(&ResolveFileProps {
            topic_id: fake_topic,
        })
        .await
        .expect("cache resolve");
    assert_eq!(resolved, payload);
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_file_service_large_payload_roundtrip() {
    let Some(ctx) = setup_ctx() else { return };
    let submit_signer: Arc<dyn Signer> = Arc::new(LocalSigner::new(
        PrivateKey::from_str_der(&ctx.operator_key).expect("key"),
    ));
    let payload = "large-payload-".repeat(20_000).into_bytes();
    let svc = HcsFileService::new(&ctx.client, ctx.network_name.clone(), None);
    let topic_id = svc
        .submit_file(SubmitFileProps {
            payload: payload.clone(),
            submit_key_signer: Arc::clone(&submit_signer),
            wait_for_visibility: false,
            wait_timeout_ms: None,
        })
        .await
        .expect("submit large");
    mirror_wait().await;
    let resolved = resolve_file_with_retries(&svc, topic_id, 12)
        .await
        .expect("resolve large");
    assert_eq!(resolved, payload);
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_file_service_invalid_topic_memo_errors() {
    let Some(ctx) = setup_ctx() else { return };
    let topic_id = HcsTopic::create_with_memo(&ctx.client, "plain-memo")
        .await
        .expect("create");
    let svc = HcsFileService::new(&ctx.client, ctx.network_name.clone(), None);
    assert!(
        svc.resolve_file(&ResolveFileProps {
            topic_id: topic_id.to_string(),
        })
        .await
        .is_err()
    );
}

#[tokio::test]
#[serial_test::serial]
async fn hcs_file_service_checksum_mismatch_errors() {
    let Some(ctx) = setup_ctx() else { return };
    let payload = b"checksum-target".to_vec();
    let memo = format!("{}{}", "a".repeat(64), HCS1_MEMO_PATTERN);
    let topic_id = HcsTopic::create_with_memo(&ctx.client, &memo)
        .await
        .expect("create");
    let chunks = chunk_payload(&payload);
    for (o, c) in chunks.iter().enumerate() {
        let msg = serde_json::to_string(&ChunkMessage { o, c: c.clone() }).expect("json");
        HcsMessage::submit(&ctx.client, topic_id, msg.into_bytes(), None)
            .await
            .expect("submit chunk");
    }
    mirror_wait().await;
    let svc = HcsFileService::new(&ctx.client, ctx.network_name.clone(), None);
    assert!(
        svc.resolve_file(&ResolveFileProps {
            topic_id: topic_id.to_string(),
        })
        .await
        .is_err()
    );
}

#[tokio::test]
#[serial_test::serial]
async fn hedera_hcs_service_end_to_end_and_network_selection() {
    let Some(ctx) = setup_ctx() else { return };
    let client_service = HederaClientService::new(service_config_single(&ctx)).expect("service");
    let svc = HederaHcsService::new(client_service, Some(HcsCacheService::with_defaults()));
    let topic_id = svc.create_topic(None).await.expect("create");
    svc.submit_message(None, topic_id, b"orchestrator".to_vec(), None)
        .await
        .expect("submit");
    mirror_wait().await;
    let messages = svc
        .get_topic_messages(
            None,
            GetTopicMessagesProps {
                topic_id,
                from_time: Some(OffsetDateTime::UNIX_EPOCH),
                to_time: None,
                limit: Some(10),
                max_idle_seconds: Some(12),
            },
        )
        .await
        .expect("messages");
    assert!(!messages.is_empty());

    let dual_client_service =
        HederaClientService::new(service_config_dual(&ctx)).expect("dual svc");
    let dual_svc = HederaHcsService::new(dual_client_service, None);
    let tid_a = dual_svc
        .create_topic(Some(&ctx.network_name))
        .await
        .expect("create a");
    let tid_b = dual_svc
        .create_topic(Some("customnet"))
        .await
        .expect("create b");
    assert_ne!(tid_a.to_string(), tid_b.to_string());
}
