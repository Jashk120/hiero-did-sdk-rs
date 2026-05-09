# API Reference

This document summarizes the public API exposed by each crate in this workspace.

## `hiero-did-core`

Shared data model, key utilities, DID parsing model, and error type.

### DID types

- `Network`: `Mainnet | Testnet`
  - parse with `"mainnet".parse::<Network>()`
- `HederaDid`
  - `HederaDid::new(network, base58_key, topic_id)`
  - `to_did_string()`
  - `root_key_id()`
  - `Display` and `FromStr` implemented

Example:

```rust
use hiero_did_core::did::{HederaDid, Network};

let did = HederaDid::new(
    Network::Testnet,
    "BASE58_PUBKEY".to_string(),
    "0.0.12345".to_string(),
);

let did_string = did.to_string();
let parsed: HederaDid = did_string.parse()?;
assert_eq!(did, parsed);
```

### Key utility

- `KeysUtility::from_bytes(Vec<u8>)`
- `KeysUtility::from_base58(&str) -> Result<_, DIDError>`
- `KeysUtility::from_multibase(&str) -> Result<_, DIDError>`
- `to_base58() -> String`
- `to_multibase() -> String`
- `to_bytes() -> &[u8]`

### DID document models

- `DIDDocument`
- `VerificationMethod`
- `Service`
- `DIDResolution`
- `DIDDocumentMetadata`
- `DIDResolutionMetadata`
- `KeyCapabilityMethod`

These are serde-serializable and intended for DID document building and resolution output.

### Errors

- `DIDError` variants:
  - `InvalidDid`
  - `InvalidArgument`
  - `InvalidSignature`
  - `InvalidMultibase`
  - `NotFound`
  - `InternalError`
  - `SerializationError`

## `hiero-did-method`

Validation and parsing helpers.

- `parse_did(did: &str) -> Result<HederaDid, DIDError>`
- `is_hedera_did(s: &str) -> bool`
- `is_topic_id(s: &str) -> bool`

Example:

```rust
use hiero_did_method::{is_hedera_did, parse_did};

let s = "did:hedera:testnet:abc_0.0.123";
if is_hedera_did(s) {
    let did = parse_did(s)?;
    println!("{}", did.topic_id);
}
```

## `hiero-did-messages`

Message/envelope/event types for DID operations over HCS.

### Types

- `HcsEnvelope { message, signature }`
- `HcsMessage { timestamp, operation, did, event }`
- `DIDOwnerEvent`, `DIDOwnerEventData`, `DIDEvent`

### Builder

- `DIDOwnerMessage::new(did, public_key_bytes, controller)`
- `to_hcs_message() -> Result<HcsMessage, DIDError>`
- `message_bytes() -> Result<Vec<u8>, DIDError>`
- `to_payload(signature: &[u8]) -> Result<String, DIDError>`

Example:

```rust
use hiero_did_messages::DIDOwnerMessage;

let msg = DIDOwnerMessage::new(did, public_key_bytes, None);
let bytes_to_sign = msg.message_bytes()?;
let payload_json = msg.to_payload(&signature_bytes)?;
```

## `hiero-did-signer`

Internal Ed25519 sign/verify helpers.

### Signer

- `InternalSigner::from_bytes(&[u8; 32]) -> Result<Self, DIDError>`
- `InternalSigner::from_raw_bytes(&[u8]) -> Result<Self, DIDError>`
- `sign(message: &[u8]) -> Vec<u8>`
- `verifying_key_bytes() -> Vec<u8>`

### Verifier

- `InternalVerifier::from_bytes(&[u8]) -> Result<Self, DIDError>`
- `verify(message: &[u8], signature: &[u8]) -> Result<bool, DIDError>`

## `hiero-did-hcs`

Hedera client wrappers and topic operations.

### Client helper

- `HcsClient::for_testnet()`
- `HcsClient::for_mainnet()`
- `set_operator(account_id, private_key)`
- `for_testnet_with_operator(account_id, private_key) -> Result<Self, DIDError>`

### Topic helper

- `HcsTopic::create(client) -> Result<TopicId, DIDError>`
- `HcsTopic::create_with_memo(client, memo) -> Result<TopicId, DIDError>`
- `HcsTopic::submit(client, topic_id, message) -> Result<SubmitMessageResult, DIDError>`

## `hiero-did-registrar`

High-level DID creation.

- `create::create_did(client, network, controller) -> Result<CreateDIDResult, DIDError>`
- `CreateDIDResult { did, private_key_bytes, public_key_bytes }`

Use this as the primary entrypoint for creation workflows.

See `create-did.md` for detailed creation guidance.

## `hiero-did-resolver`

Mirror node retrieval and DID document reconstruction.

### Mirror node client

- `MirrorNodeClient::for_testnet()`
- `MirrorNodeClient::for_mainnet()`
- `get_topic_messages(topic_id) -> Result<Vec<String>, DIDError>`

### DID document builder

- `DidDocumentBuilder::from(messages: Vec<String>) -> Self`
- `resolve(&self, did: &HederaDid) -> Result<DIDResolution, DIDError>`

Example:

```rust
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient};

let mirror = MirrorNodeClient::for_testnet();
let messages = mirror.get_topic_messages("0.0.12345").await?;
let resolution = DidDocumentBuilder::from(messages).resolve(&did).await?;
println!("{}", resolution.did_document.id);
```

## End-to-End Example

```rust
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient};

let created = create_did(&client, Network::Testnet, None).await?;
tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

let mirror = MirrorNodeClient::for_testnet();
let messages = mirror.get_topic_messages(&created.did.topic_id).await?;
let resolution = DidDocumentBuilder::from(messages)
    .resolve(&created.did)
    .await?;

assert_eq!(resolution.did_document.id, created.did.to_string());
```
