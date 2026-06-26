# Resolve DID Guide

This guide covers DID resolution and topic reading with `hiero-did-resolver`.

## Quick Start

The simplest way to resolve a DID is the top-level `resolve_did` function. It auto-selects a `MirrorNodeClient` based on the network in the DID string:

```rust
use hiero_did_resolver::resolve_did;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let resolution = resolve_did("did:hedera:testnet:<base58key>_0.0.12345", None).await?;
println!("Document id: {}", resolution.did_document.id);
# Ok(())
# }
```

## TopicReader Abstraction

Resolution is decoupled from transport through the `TopicReader` trait:

```rust
#[async_trait]
pub trait TopicReader: Send + Sync {
    async fn get_topic_messages(&self, topic_id: &str) -> Result<Vec<String>, DIDError>;
}
```

Three implementations exist:

| Reader | Transport | When to use |
|---|---|---|
| `MirrorNodeClient` | REST (mirror-node HTTP API) | Default. Works in most environments. |
| `GrpcTopicReader` | gRPC (Hedera mirror subscription) | When REST is unavailable or you want stream semantics. |
| `HcsTopicReader` | gRPC via `HederaHcsService` | When you already have an `HederaHcsService` and want to reuse its in-process cache. |

## MirrorNodeClient

REST mirror-node client with built-in pagination and polling helpers.

```rust
use hiero_did_resolver::MirrorNodeClient;

let mirror = MirrorNodeClient::for_testnet();
// or
let mirror = MirrorNodeClient::for_mainnet();
// or
let mirror = MirrorNodeClient::for_local();
// or, auto-select from HEDERA_NETWORK env var:
let mirror = MirrorNodeClient::from_env();
```

### Polling Helpers

`wait_for_mirror` polls until at least one message appears on a topic. Use after create or deactivate operations where a single message is expected:

```rust
# async fn example(mirror: &hiero_did_resolver::MirrorNodeClient) -> Result<(), Box<dyn std::error::Error>> {
let messages = mirror.wait_for_mirror("0.0.12345", 30).await?;
# Ok(())
# }
```

`wait_for_mirror_stable` polls until no new messages arrive for a configurable window. Use after update flows where multiple messages are submitted:

```rust
# async fn example(mirror: &hiero_did_resolver::MirrorNodeClient) -> Result<(), Box<dyn std::error::Error>> {
let messages = mirror.wait_for_mirror_stable("0.0.12345", 2000, 60).await?;
# Ok(())
# }
```

## GrpcTopicReader

Reader backed by gRPC mirror-node subscription (via `hiero_did_hcs::HcsMessage`). Use when REST access is unavailable or you want stronger ordering guarantees.

```rust
use hiero_did_resolver::GrpcTopicReader;

// No operator — public topics only
let reader = GrpcTopicReader::for_testnet();

// With operator — for access-controlled topics
use hiero_did_hcs::HcsClient;
use hiero_sdk::{AccountId, PrivateKey};
use std::str::FromStr;

let account_id = AccountId::from_str("0.0.12345").unwrap();
let private_key = PrivateKey::from_str_der("<DER_PRIVATE_KEY>").unwrap();

let hcs_client = HcsClient::for_testnet_with_operator(account_id, private_key).unwrap();
let reader = GrpcTopicReader::for_testnet_with_client(hcs_client);
```

Local-node support:

```rust
# use hiero_did_hcs::HcsClient;
# use hiero_did_resolver::GrpcTopicReader;
# use hiero_sdk::{AccountId, PrivateKey};
# use std::str::FromStr;
# let account_id = AccountId::from_str("0.0.12345").unwrap();
# let private_key = PrivateKey::from_str_der("<KEY>").unwrap();
let hcs_client = HcsClient::for_local_node_with_operator(account_id, private_key).unwrap();
let reader = GrpcTopicReader::for_local_node_with_client(hcs_client);
```

## HcsTopicReader

Reader backed by `HederaHcsService`, reusing its shared moka cache. Use when you already have an `HederaHcsService` in scope (e.g. from the registrar pipeline):

```rust
use std::sync::Arc;
use hiero_did_client::{
    HederaClientConfiguration, HederaClientService, HederaNetwork, NetworkConfig,
};
use hiero_did_hcs::HederaHcsService;
use hiero_did_resolver::HcsTopicReader;

let config = HederaClientConfiguration {
    networks: vec![NetworkConfig {
        network: HederaNetwork::Testnet,
        operator_id: "0.0.123".into(),
        operator_key: "<DER_PRIVATE_KEY>".into(),
    }],
};
let client_service = HederaClientService::new(config).unwrap();
let hcs_service = Arc::new(HederaHcsService::new(client_service, None));
let reader = HcsTopicReader::new(hcs_service, Some("testnet".to_string()));
```

## Resolving with DidDocumentBuilder

For lower-level control, use `DidDocumentBuilder` directly:

```rust
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient, TopicReader};
use hiero_did_core::HederaDid;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mirror = MirrorNodeClient::for_testnet();
let did: HederaDid = "did:hedera:testnet:<base58key>_0.0.12345".parse()?;

// From any TopicReader:
let resolution = DidDocumentBuilder::from_topic_reader(&mirror, &did.topic_id)
    .await?
    .resolve(&did)
    .await?;

// Or from pre-fetched messages:
let messages = mirror.get_topic_messages(&did.topic_id).await?;
let resolution = DidDocumentBuilder::from(messages).resolve(&did).await?;
# Ok(())
# }
```

## Representation Negotiation

Use `represent()` to render a resolved document in a specific format:

```rust
use hiero_did_core::Accept;
use hiero_did_resolver::represent;
# use hiero_did_core::{DIDResolution, DIDDocument, DIDDocumentMetadata, DIDResolutionMetadata};

# fn example(resolution: &DIDResolution) -> Result<(), Box<dyn std::error::Error>> {
// JSON (application/did+json)
let json = represent(resolution, Accept::DidJson)?;

// JSON-LD (application/did+ld+json)
let json_ld = represent(resolution, Accept::DidLdJson)?;

// Full resolution envelope
let full = represent(resolution, Accept::DidResolution)?;

// CBOR (application/did+cbor)
let cbor = represent(resolution, Accept::DidCbor)?;
# Ok(())
# }
```

Supported `Accept` values:

| `Accept` variant | Content type | Output |
|---|---|---|
| `DidJson` | `application/did+json` | DID document only as JSON |
| `DidLdJson` | `application/did+ld+json` | DID document with `@context` as JSON |
| `DidResolution` | Full resolution envelope | DID document + metadata as JSON |
| `DidCbor` | `application/did+cbor` | DID document as CBOR bytes |

## Typical Errors

- `InvalidDid`: malformed DID string.
- `InvalidArgument`: invalid topic ID format.
- `NotFound`: no owner message found on the topic.
- `InternalError`: mirror/gRPC fetch or timeout failures.
- `SerializationError`: JSON/CBOR encode/decode failures.

## Related APIs

- Create DID: [`create-did.md`](./create-did.md)
- Dereference DID URL: [`dereference-did.md`](./dereference-did.md)
- Full API surface: [`api-reference.md`](./api-reference.md)
