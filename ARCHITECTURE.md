# Architecture: `hiero-did-sdk-rs`

## 1. Scope

This workspace provides:

- `did:hedera` creation (`registrar`) and resolution (`resolver`).
- Hedera client configuration (`client`) and HCS topic/message/file operations (`hcs`).
- Shared DID/domain primitives (`core`, `method`, `messages`, `signer`).

## 2. Workspace Topology

Crates in `Cargo.toml`:

- `hiero-did-core`
- `hiero-did-method`
- `hiero-did-messages`
- `hiero-did-signer`
- `hiero-did-client`
- `hiero-did-hcs`
- `hiero-did-registrar`
- `hiero-did-resolver`
- `hiero-did-sdk` (re-export layer)

High-level dependency direction:

```text
hiero-did-sdk (re-export only)
        |
        +--> registrar -----> hcs -------> client
        |         |            |            |
        |         +--> messages            +--> core
        |         +--> signer
        |         +--> core
        |
        +--> resolver ------> messages
        |         |            |
        |         +--> signer  +--> core
        |
        +--> hcs -----------> core
        +--> client --------> core
        +--> method --------> core
        +--> signer --------> core
        +--> messages ------> core
```

## 3. Crate Responsibilities

### 3.1 `hiero-did-core`

Canonical DID model and shared structures:

- `did`: `Network`, `HederaDid`, `DID_METHOD`, `DID_ROOT_KEY_ID`.
- `document`: DID document + resolution metadata models.
- `keys`: base58 and multibase utilities.
- `error`: `DIDError`.
- `signer`: crate-agnostic `Signer` trait.

### 3.2 `hiero-did-method`

Thin helpers over `HederaDid` parsing:

- `parse_did`
- `is_hedera_did`
- `is_topic_id`

### 3.3 `hiero-did-messages`

Wire-level message models used on HCS:

- `HcsMessage` (signed payload)
- `HcsEnvelope` (payload + signature)
- `DIDOwnerMessage` builder
- Owner event structs (`DIDOwnerEvent*`)

### 3.4 `hiero-did-signer`

Local Ed25519 boundary:

- `InternalSigner` for private-key signing.
- `InternalVerifier` for signature validation.

### 3.5 `hiero-did-client`

Configurable network-aware client factory:

- `HederaClientConfiguration` with one or more `NetworkConfig` entries.
- Supports `Mainnet`, `Testnet`, `Previewnet`, `LocalNode`, and `Custom` networks.
- `HederaClientService::get_client` and `with_client` for operation-scoped clients.

### 3.6 `hiero-did-hcs`

HCS primitives and service façade:

- Topic ops (`HcsTopic`): create/update/delete/info/submit.
- Message ops (`HcsMessage`): submit + fetch via mirror stream.
- File ops (`HcsFileService`): HCS-1 style chunked, compressed payload publish/resolve.
- `HederaHcsService`: combines `HederaClientService` + optional `HcsCacheService`.

### 3.7 `hiero-did-registrar`

Creation orchestration:

- `create_did(client, network, controller)`
- Generates keypair, creates topic, signs owner event, submits initial envelope.

### 3.8 `hiero-did-resolver`

Resolution orchestration:

- `MirrorNodeClient` fetches topic messages from mirror node REST API.
- `DidDocumentBuilder` folds validated events into a DID document.
- Signature verification is performed per message before applying events.

### 3.9 `hiero-did-sdk`

Convenience import surface by re-exporting core/method/messages/signer/hcs/registrar/resolver.

## 4. Runtime Flows

### 4.1 Create DID

1. Generate Ed25519 keypair.
2. Create HCS topic.
3. Build DID: `did:hedera:<network>:<base58key>_<topicId>`.
4. Build owner message and sign serialized `HcsMessage` bytes.
5. Submit envelope to topic.
6. Return DID and raw key bytes.

### 4.2 Resolve DID

1. Fetch topic message history from mirror node.
2. Decode envelopes and filter by target DID.
3. Verify signatures against event-derived public key.
4. Apply valid owner event(s), track timestamps and deactivation.
5. Emit `DIDResolution` with metadata.

### 4.3 HCS Service Usage

1. Build `HederaClientConfiguration`.
2. Create `HederaClientService`.
3. Optionally attach `HcsCacheService`.
4. Use `HederaHcsService` for topic/message/file operations by network name.

## 5. Error Model

All crates map failures to `hiero_did_core::DIDError`:

- `InvalidDid`
- `InvalidArgument`
- `InvalidSignature`
- `InvalidMultibase`
- `NotFound`
- `InternalError`
- `SerializationError`

## 6. Testing Strategy

- Unit tests in each crate for parsing, key conversion, signing, and event application.
- Integration tests:
  - `registrar/tests/integration_test.rs`: create and resolve DID against testnet.
  - `hcs/tests/integration_hcs.rs`: topic/message/file and service-level HCS behavior.
  - `client/tests/client_service_integration.rs`: configuration + client setup behavior.
