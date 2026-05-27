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
- Constants: `DID_METHOD`, `DID_ROOT_KEY_ID`
- `did_url::HederaDidUrl`
- Fields: `did`, `path`, `params`, `fragment`
- `FromStr` parser for DID URL strings
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

HCS envelope and DID event payload helpers.

- `HcsEnvelope { message, signature }`
- `HcsMessage { timestamp, operation, did, event }`
- Create: `DIDOwnerMessage`
- Update:
- `DIDAddVerificationMethodMessage`
- `DIDRemoveVerificationMethodMessage`
- `DIDAddServiceMessage`
- `DIDRemoveServiceMessage`
- Deactivate: `DIDDeactivateMessage`
- Event models in `events.rs`

## `hiero-did-signer`

Ed25519 sign/verify helpers and optional external custody signing.

- `InternalSigner`
- `from_bytes(&[u8; 32]) -> Result<Self, DIDError>`
- `from_raw_bytes(&[u8]) -> Result<Self, DIDError>`
- `sign(&[u8]) -> Vec<u8>`
- `verifying_key_bytes() -> Vec<u8>`
- `InternalVerifier`
- `from_bytes(&[u8]) -> Result<Self, DIDError>`
- `verify(message: &[u8], signature: &[u8]) -> Result<bool, DIDError>`
- With the `vault` feature:
- `VaultSigner`
- `new(VaultSignerConfig) -> Result<Self, DIDError>`
- Implements `hiero_did_core::Signer`
- `VaultSignerConfig`
- `new(vault_url, auth, key_name) -> Self`
- Defaults `mount_path` to `transit`
- `VaultAuth`
- `Token(String)`
- `AppRole { role_id, secret_id }`

## `hiero-did-client`

Config-driven Hedera SDK client construction.

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

Hedera HCS helper layer and service facade.

- `HcsClient`
- `for_testnet()`, `for_mainnet()`
- `set_operator(account_id, private_key)`
- `for_testnet_with_operator(account_id, private_key)`
- Topic operations (`HcsTopic`)
- `create`, `create_with_memo`, `create_with_props`
- `update`, `delete`, `delete_with_props`, `get_info`
- `submit`
- Topic/message types
- `CreateTopicProps`, `UpdateTopicProps`, `DeleteTopicProps`, `TopicInfo`
- `CreateTopicProps.submit_key_signer: Option<Arc<dyn Signer>>`
- `CreateTopicProps.admin_key_signer: Option<Arc<dyn Signer>>`
- `UpdateTopicProps.admin_key_signer: Arc<dyn Signer>`
- `GetTopicMessagesProps`, `TopicMessageData`, `SubmitMessageResult`
- `HcsMessage::submit`, `HcsMessage::get_topic_messages`, `HcsMessage::get_topic_messages_with_cache`
- File operations
- `SubmitFileProps`, `ResolveFileProps`
- `HcsFileService::submit_file`, `HcsFileService::resolve_file`
- Service facade (`HederaHcsService`)
- `create_topic`, `create_topic_with_memo`, `create_topic_with_props`
- `update_topic`, `delete_topic`, `delete_topic_with_props`, `get_topic_info`
- `submit_message`, `get_topic_messages`, `submit_file`, `resolve_file`
- Cache
- `HcsCacheService::new(max_size)`
- `HcsCacheService::with_defaults()`

## `hiero-did-registrar`

High-level DID write operations.

- `create::create_did(client, network, controller) -> Result<CreateDIDResult, DIDError>`
- `create::create_did_with_signer(client, network, controller, signer) -> Result<CreateDIDWithSignerResult, DIDError>`
- `update::update_did(client, did, private_key_bytes, updates) -> Result<UpdateDIDResult, DIDError>`
- `update::update_did_with_signer(client, did, signer, updates) -> Result<UpdateDIDResult, DIDError>`
- `deactivate::deactivate_did(client, did, private_key_bytes) -> Result<DeactivateDIDResult, DIDError>`
- `deactivate::deactivate_did_with_signer(client, did, signer) -> Result<DeactivateDIDResult, DIDError>`
- `CreateDIDResult { did, private_key_bytes, public_key_bytes }`
- `CreateDIDWithSignerResult { did, public_key_bytes }`
- `UpdateDIDResult { did, operations_applied }`
- `DeactivateDIDResult { did, did_document }`
- Update helper types
- `DIDUpdateOperation`
- `AddVerificationMethod`, `RemoveVerificationMethod`, `AddService`, `RemoveService`
- `VerificationMethodProperty`

## `hiero-did-resolver`

Mirror-node retrieval, DID resolution, and DID URL dereference.

- `MirrorNodeClient`
- `for_testnet()`
- `for_mainnet()`
- `get_topic_messages(topic_id: &str) -> Result<Vec<String>, DIDError>`
- `DidDocumentBuilder`
- `from(messages: Vec<String>) -> DidDocumentBuilder`
- `resolve(&self, did: &HederaDid) -> Result<DIDResolution, DIDError>` (async)
- DID URL dereference
- `dereference::DereferencedResource`
- `Document(DIDDocument)`
- `VerificationMethod(VerificationMethod)`
- `Service(Service)`
- `dereference::dereference_did(did_url: &HederaDidUrl, messages: Vec<String>) -> Result<DereferencedResource, DIDError>` (async)

## `hiero-did-anoncreds`

AnonCreds registry on top of HCS.

- `HederaAnonCredsRegistry`
- `register_schema`, `get_schema`
- `register_credential_definition`, `get_credential_definition`
- `register_revocation_registry_definition`, `get_revocation_registry_definition`
- `register_revocation_status_list`, `get_revocation_status_list`

## `hiero-did-sdk`

Umbrella crate re-exports:

- `hiero_did_sdk::core`
- `hiero_did_sdk::method`
- `hiero_did_sdk::messages`
- `hiero_did_sdk::signer`
- `hiero_did_sdk::client`
- `hiero_did_sdk::hcs`
- `hiero_did_sdk::registrar`
- `hiero_did_sdk::resolver`
- `hiero_did_sdk::anoncreds`
