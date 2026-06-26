# Dereference DID Guide

This guide covers DID URL parsing and dereference with `hiero-did-core` and `hiero-did-resolver`.

## Quick Start

The simplest way to dereference a DID URL is the top-level `dereference_did_url` function. It auto-selects a `MirrorNodeClient` based on the network in the DID:

```rust
use hiero_did_resolver::dereference_did_url;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let resource = dereference_did_url(
    "did:hedera:testnet:<base58key>_0.0.12345#did-root-key",
    None,
).await?;
# Ok(())
# }
```

Pass a custom `TopicReader` to control how messages are fetched:

```rust
use hiero_did_resolver::{GrpcTopicReader, dereference_did_url};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let reader = GrpcTopicReader::for_testnet();
let resource = dereference_did_url(
    "did:hedera:testnet:<base58key>_0.0.12345#did-root-key",
    Some(&reader),
).await?;
# Ok(())
# }
```

## APIs

```rust
impl std::str::FromStr for hiero_did_core::HederaDidUrl
```

```rust
// Convenience — auto-selects reader from DID network
pub async fn dereference_did_url(
    did_url: &str,
    reader: Option<&dyn TopicReader>,
) -> Result<DereferencedResource, DIDError>
```

```rust
// With explicit Accept format
pub async fn dereference_did_url_with_accept(
    did_url: &str,
    reader: Option<&dyn TopicReader>,
    accept: Accept,
) -> Result<DereferencedResource, DIDError>
```

```rust
// Lower-level — caller supplies pre-fetched messages
pub async fn dereference_did(
    did_url: &HederaDidUrl,
    messages: Vec<String>,
) -> Result<DereferencedResource, DIDError>
```

```rust
// Lower-level with Accept format
pub async fn dereference_did_with_accept(
    did_url: &HederaDidUrl,
    messages: Vec<String>,
    accept: Accept,
) -> Result<DereferencedResource, DIDError>
```

## Supported Inputs

- Bare DID URL (no fragment): returns whole DID document.
- DID URL with `#fragment`: returns matching verification method or service.

Current limitation:

- Path and query params are parsed by `HederaDidUrl`, but `dereference_did` currently returns `InvalidArgument` when either is present.

## DereferencedResource Variants

```rust
pub enum DereferencedResource {
    Document(DIDDocument),
    VerificationMethod(VerificationMethod),
    Service(Service),
    Represented(RepresentedDocument),
}
```

The `Represented` variant is returned when a non-default `Accept` format is requested via `dereference_did_url_with_accept` or `dereference_did_with_accept` for bare DID URLs.

## End-to-End Example

```rust
use hiero_did_resolver::{DereferencedResource, dereference_did_url};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resource = dereference_did_url(
        "did:hedera:testnet:<base58key>_0.0.12345#did-root-key",
        None,
    ).await?;

    match resource {
        DereferencedResource::Document(doc) => {
            println!("Resolved document id: {}", doc.id);
        }
        DereferencedResource::VerificationMethod(vm) => {
            println!("Verification method id: {}", vm.id());
        }
        DereferencedResource::Service(svc) => {
            println!("Service id: {}", svc.id);
        }
        DereferencedResource::Represented(rep) => {
            println!("Represented document in requested format");
        }
    }

    Ok(())
}
```

## Pre-Fetched Messages Example

When you already have topic messages (e.g. from an `HcsTopicReader` cache hit):

```rust
use hiero_did_core::HederaDidUrl;
use hiero_did_resolver::{dereference::dereference_did, DereferencedResource, MirrorNodeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let did_url: HederaDidUrl =
        "did:hedera:testnet:<base58key>_0.0.12345#did-root-key".parse()?;

    let mirror = MirrorNodeClient::for_testnet();
    let messages = mirror.get_topic_messages(&did_url.did.topic_id).await?;

    let resource = dereference_did(&did_url, messages).await?;

    match resource {
        DereferencedResource::Document(doc) => {
            println!("Resolved document id: {}", doc.id);
        }
        DereferencedResource::VerificationMethod(vm) => {
            println!("Verification method id: {}", vm.id());
        }
        DereferencedResource::Service(svc) => {
            println!("Service id: {}", svc.id);
        }
        DereferencedResource::Represented(_) => {}
    }

    Ok(())
}
```

## Fragment Matching Behavior

`dereference_did` builds a full identifier as:

`<did>#<fragment>`

It then searches:

- `didDocument.verificationMethod[].id`
- `didDocument.service[].id`

If no match exists, it returns `DIDError::NotFound`.

## Typical Errors

- `InvalidDid`: malformed DID URL string.
- `InvalidArgument`: path/query params provided (not supported by dereference yet).
- `NotFound`: fragment not found in verification methods/services.
- `InternalError`: mirror/gRPC fetch or serialization failures upstream.

## Related APIs

- Resolve full document:
  - `resolve_did(did, reader)` — convenience function
  - `DidDocumentBuilder::from(messages).resolve(&did)` — lower-level
  - `DidDocumentBuilder::from_topic_reader(reader, topic_id)` — transport-agnostic
- Fetch topic messages:
  - `MirrorNodeClient::get_topic_messages(topic_id)`
  - Any `TopicReader` implementation
- See [`resolve-did.md`](./resolve-did.md) for the full resolution guide.
