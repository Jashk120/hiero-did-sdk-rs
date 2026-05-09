# API Reference

Public API summary across workspace crates.

## `hiero-did-core`

Shared data model, DID types, errors, and key utilities.

### DID and network types

- `did::Network`: `Mainnet | Testnet`
- `did::HederaDid`
  - `HederaDid::new(network, base58_key, topic_id)`
  - `to_did_string()`
  - `root_key_id()`
  - `Display` and `FromStr`

### Key utility

- `KeysUtility::from_bytes(Vec<u8>)`
- `KeysUtility::from_base58(&str) -> Result<_, DIDError>`
- `KeysUtility::from_multibase(&str) -> Result<_, DIDError>`
- `to_base58() -> String`
- `to_multibase() -> String`
- `to_bytes() -> &[u8]`

### DID document models

- `DIDDocument`
- `VerificationMethod` (+ Base58/Multibase variants)
- `Service`
- `DIDResolution`
- `DIDDocumentMetadata`
- `DIDResolutionMetadata`
- `KeyCapabilityMethod`

### Error type

- `DIDError`
  - `InvalidDid`
  - `InvalidArgument`
  - `InvalidSignature`
  - `InvalidMultibase`
  - `NotFound`
  - `InternalError`
  - `SerializationError`

## `hiero-did-method`

DID parser and validators.

- `parse_did(did: &str) -> Result<HederaDid, DIDError>`
- `is_hedera_did(s: &str) -> bool`
- `is_topic_id(s: &str) -> bool`

## `hiero-did-messages`

HCS envelope and DID owner event payload helpers.

### Types

- `HcsEnvelope { message, signature }`
- `HcsMessage { timestamp, operation, did, event }`
- `DIDOwnerEvent`, `DIDOwnerEventData`, `DIDEvent`

### Builder

- `DIDOwnerMessage::new(did, public_key_bytes, controller)`
- `to_hcs_message() -> Result<HcsMessage, DIDError>`
- `message_bytes() -> Result<Vec<u8>, DIDError>`
- `to_payload(signature: &[u8]) -> Result<String, DIDError>`

## `hiero-did-signer`

Ed25519 sign/verify helpers.

- `InternalSigner::from_bytes(&[u8; 32]) -> Result<Self, DIDError>`
- `InternalSigner::from_raw_bytes(&[u8]) -> Result<Self, DIDError>`
- `sign(message: &[u8]) -> Vec<u8>`
- `verifying_key_bytes() -> Vec<u8>`

- `InternalVerifier::from_bytes(&[u8]) -> Result<Self, DIDError>`
- `verify(message: &[u8], signature: &[u8]) -> Result<bool, DIDError>`

## `hiero-did-hcs`

Hedera client and topic helpers.

### Client helper

- `HcsClient::for_testnet() -> HcsClient`
- `HcsClient::for_mainnet() -> HcsClient`
- `set_operator(account_id, private_key)`
- `for_testnet_with_operator(account_id, private_key) -> Result<HcsClient, DIDError>`

### Topic helper

- `HcsTopic::create(client) -> Result<TopicId, DIDError>`
- `HcsTopic::create_with_memo(client, memo) -> Result<TopicId, DIDError>`
- `HcsTopic::submit(client, topic_id, message) -> Result<SubmitMessageResult, DIDError>`

`SubmitMessageResult` fields:

- `topic_id: String`
- `sequence_number: Option<u64>`

## `hiero-did-registrar`

High-level DID creation.

- `create::create_did(client, network, controller) -> Result<CreateDIDResult, DIDError>`
- `CreateDIDResult { did, private_key_bytes, public_key_bytes }`

## `hiero-did-resolver`

Mirror node retrieval and DID resolution.

### Mirror client

- `MirrorNodeClient::for_testnet() -> MirrorNodeClient`
- `MirrorNodeClient::for_mainnet() -> MirrorNodeClient`
- `get_topic_messages(topic_id: &str) -> Result<Vec<String>, DIDError>`

### DID document builder

- `DidDocumentBuilder::from(messages: Vec<String>) -> DidDocumentBuilder`
- `resolve(&self, did: &HederaDid) -> Result<DIDResolution, DIDError>` (async)

## `hiero-did-sdk`

Umbrella crate that re-exports all workspace crates:

- `hiero_did_sdk::core`
- `hiero_did_sdk::method`
- `hiero_did_sdk::messages`
- `hiero_did_sdk::signer`
- `hiero_did_sdk::hcs`
- `hiero_did_sdk::registrar`
- `hiero_did_sdk::resolver`

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
