# API Reference

Public API summary across workspace crates.

## `hiero-did-core`

Shared data model, DID types, errors, signer trait, and key utilities.

- `did::Network`: `Mainnet | Testnet`
- `did::HederaDid`
  - `new(network, base58_key, topic_id)`
  - `to_did_string()`
  - `root_key_id()`
  - `Display` and `FromStr`
- Constants
  - `DID_METHOD`
  - `DID_ROOT_KEY_ID`
- `keys::KeysUtility`
  - `from_bytes(Vec<u8>)`
  - `from_base58(&str) -> Result<_, DIDError>`
  - `from_multibase(&str) -> Result<_, DIDError>`
  - `to_base58() -> String`
  - `to_multibase() -> String`
  - `to_bytes() -> &[u8]`
- DID document models
  - `DIDDocument`, `VerificationMethod`, `Service`, `DIDResolution`, related metadata/types
- `Signer` trait
  - `public_key_bytes() -> Vec<u8>`
  - `sign_bytes(&[u8]) -> Result<Vec<u8>, DIDError>`
- `DIDError`
  - `InvalidDid`, `InvalidArgument`, `InvalidSignature`, `InvalidMultibase`, `NotFound`, `InternalError`, `SerializationError`

## `hiero-did-method`

DID parser and validators.

- `parse_did(did: &str) -> Result<HederaDid, DIDError>`
- `is_hedera_did(s: &str) -> bool`
- `is_topic_id(s: &str) -> bool`

## `hiero-did-messages`

HCS envelope and DID owner event payload helpers.

- `HcsEnvelope { message, signature }`
- `HcsMessage { timestamp, operation, did, event }`
- `DIDOwnerEvent`, `DIDOwnerEventData`
- `DIDOwnerMessage`
  - `new(did, public_key_bytes, controller)`
  - `to_hcs_message() -> Result<HcsMessage, DIDError>`
  - `message_bytes() -> Result<Vec<u8>, DIDError>`
  - `to_payload(signature: &[u8]) -> Result<String, DIDError>`

## `hiero-did-signer`

Ed25519 sign/verify helpers.

- `InternalSigner`
  - `from_bytes(&[u8; 32]) -> Result<Self, DIDError>`
  - `from_raw_bytes(&[u8]) -> Result<Self, DIDError>`
  - `sign(&[u8]) -> Vec<u8>`
  - `verifying_key_bytes() -> Vec<u8>`
- `InternalVerifier`
  - `from_bytes(&[u8]) -> Result<Self, DIDError>`
  - `verify(message: &[u8], signature: &[u8]) -> Result<bool, DIDError>`

## `hiero-did-client`

Config-driven Hedera SDK client construction.

Types:

- `HederaNetwork`: `Mainnet | Testnet | Previewnet | LocalNode | Custom(HederaCustomNetwork)`
- `HederaCustomNetwork { name, nodes, mirror_nodes }`
- `NetworkConfig { network, operator_id, operator_key }`
- `HederaClientConfiguration { networks }`
- `NetworkName { network_name: Option<String> }`
- `HederaClientService`
  - `new(config) -> Result<Self, DIDError>`
  - `get_client(network_name: Option<&str>) -> Result<Client, DIDError>`
  - `with_client(network_name, operation) -> Result<T, DIDError>` (async)

## `hiero-did-hcs`

Hedera HCS helper layer and service façade.

Core client/signer:

- `HcsClient`
  - `for_testnet()`, `for_mainnet()`
  - `set_operator(account_id, private_key)`
  - `for_testnet_with_operator(account_id, private_key)`
- `LocalSigner::new(private_key)` (`Signer` trait implementation)

Topic operations (`HcsTopic`):

- `create(client)`
- `create_with_memo(client, memo)`
- `create_with_props(client, CreateTopicProps)`
- `update(client, UpdateTopicProps)`
- `delete(client, topic_id)`
- `delete_with_props(client, DeleteTopicProps)`
- `get_info(client, topic_id)`
- `submit(client, topic_id, message)`

Topic/message types:

- `CreateTopicProps`, `UpdateTopicProps`, `DeleteTopicProps`, `TopicInfo`
- `GetTopicMessagesProps`, `TopicMessageData`, `SubmitMessageResult`
- `HcsMessage`
  - `submit(client, topic_id, message, submit_key_signer)`
  - `get_topic_messages(client, props)`
  - `get_topic_messages_with_cache(client, props, network_name, cache)`

File operations:

- `SubmitFileProps`, `ResolveFileProps`
- `HcsFileService::new(client, network_name, cache)`
- `submit_file(props) -> Result<String, DIDError>`
- `resolve_file(props) -> Result<Vec<u8>, DIDError>`

Service façade:

- `HederaHcsService::new(client_service, cache)`
- `create_topic`, `create_topic_with_memo`, `create_topic_with_props`
- `update_topic`, `delete_topic`, `delete_topic_with_props`, `get_topic_info`
- `submit_message`, `get_topic_messages`
- `submit_file`, `resolve_file`

Cache:

- `HcsCacheService::new(max_size)`
- `HcsCacheService::with_defaults()`

## `hiero-did-registrar`

High-level DID creation.

- `create::create_did(client, network, controller) -> Result<CreateDIDResult, DIDError>`
- `CreateDIDResult { did, private_key_bytes, public_key_bytes }`

## `hiero-did-resolver`

Mirror-node retrieval and DID resolution.

- `MirrorNodeClient`
  - `for_testnet()`
  - `for_mainnet()`
  - `get_topic_messages(topic_id: &str) -> Result<Vec<String>, DIDError>`
- `DidDocumentBuilder`
  - `from(messages: Vec<String>) -> DidDocumentBuilder`
  - `resolve(&self, did: &HederaDid) -> Result<DIDResolution, DIDError>` (async)

## `hiero-did-sdk`

Umbrella crate that re-exports:

- `hiero_did_sdk::core`
- `hiero_did_sdk::method`
- `hiero_did_sdk::messages`
- `hiero_did_sdk::signer`
- `hiero_did_sdk::hcs`
- `hiero_did_sdk::registrar`
- `hiero_did_sdk::resolver`
