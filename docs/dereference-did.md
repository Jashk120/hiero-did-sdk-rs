# Dereference DID Guide

This guide covers DID URL parsing and dereference with `hiero-did-core` and `hiero-did-resolver`.

## APIs

```rust
impl std::str::FromStr for hiero_did_core::HederaDidUrl
```

```rust
pub async fn dereference_did(
    did_url: &hiero_did_core::HederaDidUrl,
    messages: Vec<String>,
) -> Result<hiero_did_resolver::DereferencedResource, hiero_did_core::DIDError>
```

## Supported Inputs

- Bare DID URL (no fragment): returns whole DID document.
- DID URL with `#fragment`: returns matching verification method or service.

Current limitation:

- Path and query params are parsed by `HederaDidUrl`, but `dereference_did` currently returns `InvalidArgument` when either is present.

## End-to-End Example

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
- `InternalError`: mirror fetch/serialization failures upstream.

## Related APIs

- Resolve full document:
  - `DidDocumentBuilder::from(messages).resolve(&did)`
- Fetch topic messages:
  - `MirrorNodeClient::get_topic_messages(topic_id)`
