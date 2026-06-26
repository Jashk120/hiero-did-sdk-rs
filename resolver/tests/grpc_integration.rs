use std::env;
use std::str::FromStr;

use dotenvy::{
    from_filename,
    from_filename_override,
};
use hiero_did_client::{
    HederaClientConfiguration,
    HederaClientService,
    HederaNetwork,
    NetworkConfig,
};
use hiero_did_core::did::Network;
use hiero_did_hcs::HcsClient;
use hiero_did_registrar::create::create_did;
use hiero_did_resolver::{
    DidDocumentBuilder,
    GrpcTopicReader,
    MirrorNodeClient,
    TopicReader,
};
use hiero_sdk::{
    AccountId,
    PrivateKey,
};

// ---------------------------------------------------------------------------
// Setup helpers — identical pattern to registrar/tests/integration_test.rs
// ---------------------------------------------------------------------------

fn setup_env() {
    let _ = from_filename_override(".env.local");
    let _ = from_filename(".env");
}

fn get_network() -> Network {
    match env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string()).as_str() {
        "mainnet" => Network::Mainnet,
        "local" | "local-node" | "localhost" => Network::Testnet,
        _ => Network::Testnet,
    }
}

fn get_client_network() -> HederaNetwork {
    match env::var("HEDERA_NETWORK").unwrap_or_else(|_| "testnet".to_string()).as_str() {
        "mainnet" => HederaNetwork::Mainnet,
        "previewnet" => HederaNetwork::Previewnet,
        "local" | "local-node" | "localhost" => HederaNetwork::LocalNode,
        _ => HederaNetwork::Testnet,
    }
}

struct Ctx {
    hedera_client: hiero_sdk::Client,
    grpc_reader: GrpcTopicReader,
    mirror: MirrorNodeClient,
    network: Network,
}

/// Returns `None` if env vars are missing — tests that call this will skip
/// gracefully rather than panic in CI without credentials.
fn setup() -> Option<Ctx> {
    setup_env();

    let operator_id = env::var("HEDERA_ACCOUNT_ID").ok()?;
    let operator_key = env::var("HEDERA_PRIVATE_KEY").ok()?;

    let service = HederaClientService::new(HederaClientConfiguration {
        networks: vec![NetworkConfig {
            network: get_client_network(),
            operator_id: operator_id.clone(),
            operator_key: operator_key.clone(),
        }],
    })
    .ok()?;

    let hedera_client = service.get_client(None).ok()?;

    let account_id = AccountId::from_str(&operator_id).ok()?;
    let private_key = PrivateKey::from_str_der(&operator_key).ok()?;

    // Build GrpcTopicReader with an HcsClient that targets the *same* network
    // as hedera_client — local-node must use for_local_node_with_operator so
    // TopicMessageQuery::subscribe hits 127.0.0.1:38081, not testnet mirror.
    let grpc_reader = match get_client_network() {
        HederaNetwork::Mainnet => {
            let hcs_client = HcsClient::for_mainnet();
            hcs_client.set_operator(account_id, private_key);
            GrpcTopicReader::for_mainnet_with_client(hcs_client)
        }
        HederaNetwork::LocalNode => {
            let hcs_client =
                HcsClient::for_local_node_with_operator(account_id, private_key).ok()?;
            GrpcTopicReader::for_local_node_with_client(hcs_client)
        }
        _ => {
            let hcs_client = HcsClient::for_testnet_with_operator(account_id, private_key).ok()?;
            GrpcTopicReader::for_testnet_with_client(hcs_client)
        }
    };

    let mirror = MirrorNodeClient::from_env();
    let network = get_network();

    Some(Ctx { hedera_client, grpc_reader, mirror, network })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// GrpcTopicReader must return the same messages as MirrorNodeClient for the
/// same topic after creation.
#[tokio::test]
#[serial_test::serial]
async fn grpc_reader_returns_messages_for_created_did() {
    let Some(ctx) = setup() else { return };

    let created =
        create_did(&ctx.hedera_client, ctx.network, None).await.expect("Failed to create DID");

    // Wait for mirror to index before gRPC subscription closes
    ctx.mirror
        .wait_for_mirror(&created.did.topic_id, 30)
        .await
        .expect("Mirror timed out");

    let messages = ctx
        .grpc_reader
        .get_topic_messages(&created.did.topic_id)
        .await
        .expect("gRPC get_topic_messages failed");

    assert!(!messages.is_empty(), "gRPC reader returned no messages");
}

/// Resolving a DID via GrpcTopicReader must produce the same document id as
/// resolving via MirrorNodeClient.
#[tokio::test]
#[serial_test::serial]
async fn grpc_reader_resolves_same_document_as_mirror() {
    let Some(ctx) = setup() else { return };

    let created =
        create_did(&ctx.hedera_client, ctx.network, None).await.expect("Failed to create DID");

    // Mirror resolution (ground truth)
    let mirror_messages = ctx
        .mirror
        .wait_for_mirror(&created.did.topic_id, 30)
        .await
        .expect("Mirror timed out");

    let mirror_resolution = DidDocumentBuilder::from(mirror_messages)
        .resolve(&created.did)
        .await
        .expect("Mirror resolve failed");

    // gRPC resolution
    let grpc_messages = ctx
        .grpc_reader
        .get_topic_messages(&created.did.topic_id)
        .await
        .expect("gRPC get_topic_messages failed");

    assert!(!grpc_messages.is_empty(), "gRPC reader returned no messages");

    let grpc_resolution = DidDocumentBuilder::from(grpc_messages)
        .resolve(&created.did)
        .await
        .expect("gRPC resolve failed");

    assert_eq!(grpc_resolution.did_document.id, mirror_resolution.did_document.id);
    assert_eq!(
        grpc_resolution.did_document.verification_method,
        mirror_resolution.did_document.verification_method
    );
    assert_eq!(
        grpc_resolution.did_document_metadata.deactivated,
        mirror_resolution.did_document_metadata.deactivated
    );
}

/// GrpcTopicReader implements TopicReader — verify it can be used
/// polymorphically via &dyn TopicReader with DidDocumentBuilder.
#[tokio::test]
#[serial_test::serial]
async fn grpc_reader_works_as_dyn_topic_reader() {
    let Some(ctx) = setup() else { return };

    let created =
        create_did(&ctx.hedera_client, ctx.network, None).await.expect("Failed to create DID");

    ctx.mirror.wait_for_mirror(&created.did.topic_id, 30).await.expect("Mirror timed out");

    let reader: &dyn TopicReader = &ctx.grpc_reader;

    let resolution = DidDocumentBuilder::from_topic_reader(reader, &created.did.topic_id)
        .await
        .expect("from_topic_reader failed")
        .resolve(&created.did)
        .await
        .expect("resolve failed");

    assert_eq!(resolution.did_document.id, created.did.to_string());
    assert!(!resolution.did_document.verification_method.is_empty());
    assert_eq!(resolution.did_document_metadata.deactivated, Some(false));
}

/// Invalid topic id must return an error, not panic.
#[tokio::test]
#[serial_test::serial]
async fn grpc_reader_invalid_topic_id_returns_error() {
    let Some(ctx) = setup() else { return };

    let result = ctx.grpc_reader.get_topic_messages("not-a-topic-id").await;
    assert!(result.is_err(), "Expected error for invalid topic id");
}