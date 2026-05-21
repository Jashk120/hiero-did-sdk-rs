# Architecture: `hiero-did-sdk-rs`

## 1. Scope

This Rust workspace currently provides:

- DID write operations: create, update, deactivate (`registrar`).
- DID resolution from topic history (`resolver`).
- DID URL dereference for resolved documents/resources (`resolver` + `core::did_url`).
- Hedera client configuration (`client`) and HCS topic/message/file operations (`hcs`).
- Shared DID/domain primitives (`core`, `method`, `messages`, `signer`).
- AnonCreds registry operations on top of HCS (`anoncreds`).
- A convenience re-export layer (`sdk`).
- A local scratch binary crate for experiments (`scratch`).

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
- `hiero-did-anoncreds`
- `hiero-did-sdk` (re-export layer)
- `scratch` (local binary crate; not part of SDK surface)

High-level dependency direction:

```text
hiero-did-sdk (re-export only)
        |
        +--> registrar -----> hcs -------> client -------> core
        |         |            |
        |         +--> messages|
        |         +--> signer  |
        |         +------------+
        |
        +--> resolver ------> messages -----> core
        |         |
        |         +--> signer
        |
        +--> anoncreds -----> hcs ----------> client -----> core
        |
        +--> method --------> core
        +--> signer --------> core
        +--> client --------> core
        +--> hcs -----------> core
        +--> messages ------> core
```

## 3. Crate Responsibilities

### 3.1 `hiero-did-core`

Canonical DID model and shared structures:

- `did`: `Network`, `HederaDid`, `DID_METHOD`, `DID_ROOT_KEY_ID`.
- `did_url`: `HederaDidUrl` parser for DID URLs (`did`, `path`, `params`, `fragment`).
- `document`: DID document + resolution metadata models.
- `keys`: base58 and multibase utilities.
- `error`: `DIDError`.
- `signer`: crate-agnostic `Signer` trait.

### 3.2 `hiero-did-method`

DID parser and validators:

- `parse_did`
- `is_hedera_did`
- `is_topic_id`

### 3.3 `hiero-did-messages`

Wire-level message models used on HCS:

- `HcsMessage` (typed event envelope data)
- `HcsEnvelope` (payload + signature)
- Create family: `DIDOwnerMessage`
- Update families:
- `DIDAddVerificationMethodMessage`
- `DIDRemoveVerificationMethodMessage`
- `DIDAddServiceMessage`
- `DIDRemoveServiceMessage`
- Deactivate family: `DIDDeactivateMessage`
- Event models in `events.rs` for owner/update/service/deactivate payloads.

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

HCS primitives and service facade:

- Topic ops (`HcsTopic`): create/update/delete/info/submit.
- Message ops (`hcs::message::HcsMessage`): submit + fetch via mirror stream.
- File ops (`HcsFileService`): chunked/compressed payload publish/resolve.
- Cache (`HcsCacheService`): topic info/messages/file caching.
- `HederaHcsService`: combines `HederaClientService` + optional cache.

### 3.7 `hiero-did-registrar`

DID write orchestration:

- `create::create_did(client, network, controller)`
- `update::update_did(client, did, private_key_bytes, updates)`
- `deactivate::deactivate_did(client, did, private_key_bytes)`

The update path supports verification method and service add/remove operations via `DIDUpdateOperation`.

### 3.8 `hiero-did-resolver`

Resolution orchestration:

- `MirrorNodeClient` fetches topic messages from mirror node REST API.
- `DidDocumentBuilder` folds validated events into a DID document.
- Signature verification is performed per message before applying events.
- Resolver applies create/update/service/deactivate semantics from event stream.
- DID URL dereference is exposed via `resolver::dereference`:
  - `dereference_did(did_url, messages)` resolves from topic message history and returns either:
  - whole document (`DereferencedResource::Document`)
  - matching verification method (`DereferencedResource::VerificationMethod`)
  - matching service (`DereferencedResource::Service`)
- Current implementation is exposed as `resolver::mirror`, `resolver::builder`, and `resolver::dereference`.

### 3.9 `hiero-did-anoncreds`

AnonCreds registry layer on top of HCS service:

- `HederaAnonCredsRegistry` operations for:
- schema register/get
- credential definition register/get
- revocation registry definition register/get
- revocation status list register/get
- `types` module for AnonCreds payload models and HCS metadata.
- `utils` module for identifier building/parsing and revocation entry pack/unpack/diff helpers.

### 3.10 `hiero-did-sdk`

Convenience import surface by re-exporting:

- `core`, `method`, `messages`, `signer`, `client`, `hcs`, `registrar`, `resolver`, `anoncreds`.

### 3.11 `scratch`

Local binary crate used for ad-hoc experiments:

- Not re-exported by `hiero-did-sdk`.
- Not part of the public SDK contract.
- Can change independently without semantic compatibility guarantees.

## 4. Runtime Flows

### 4.1 Create DID

1. Generate Ed25519 keypair.
2. Create HCS topic.
3. Build DID: `did:hedera:<network>:<base58key>_<topicId>`.
4. Build owner message and sign serialized `HcsMessage` bytes.
5. Submit envelope to topic.
6. Return DID and raw key bytes.

### 4.2 Update DID

1. Parse topic ID from target DID.
2. Convert each `DIDUpdateOperation` to an update message.
3. Sign serialized message bytes with DID private key.
4. Submit each signed envelope to the same DID topic in order.
5. Return applied operation count.

Notes:

- Empty update list returns early with `operations_applied = 0`.
- `verificationMethod` updates require `public_key_multibase`.

### 4.3 Deactivate DID

1. Parse topic ID from target DID.
2. Build deactivate message (`operation = "delete"`).
3. Sign serialized message bytes with DID private key.
4. Submit signed envelope to DID topic.
5. Return tombstoned document metadata payload.

### 4.4 Resolve DID

1. Fetch topic message history from mirror node.
2. Decode envelopes and filter by target DID.
3. Establish verifier from DID owner event key material.
4. Verify signatures against that key.
5. Apply owner/update/service events; treat `operation == "delete"` as deactivation.
6. Emit `DIDResolution` with metadata.

### 4.5 HCS Service Usage

1. Build `HederaClientConfiguration`.
2. Create `HederaClientService`.
3. Optionally attach `HcsCacheService`.
4. Use `HederaHcsService` for topic/message/file operations by network name.

### 4.6 Dereference DID URL

1. Parse DID URL into `HederaDidUrl`.
2. Obtain topic messages for the DID topic (typically via `MirrorNodeClient`).
3. Call `dereference_did(&did_url, messages)`.
4. If URL has no fragment, return full `DIDDocument`.
5. If URL has a fragment, match `did#fragment` against verification methods and services.

### 4.7 AnonCreds Registry Usage

1. Build `HederaClientService` and `HederaHcsService`.
2. Construct `HederaAnonCredsRegistry`.
3. Register/read AnonCreds objects via DID+topic-backed identifiers.

## 5. Error Model

All crates map failures to `hiero_did_core::DIDError` variants:

- `InvalidDid`
- `InvalidArgument`
- `InvalidSignature`
- `InvalidMultibase`
- `NotFound`
- `InternalError`
- `SerializationError`

## 6. Testing Strategy

- Unit tests per crate for parsing, key conversion, signing, event application, and registry utilities.
- Integration tests currently include:
- `client/tests/client_service_integration.rs`
- `hcs/tests/integration_hcs.rs`
- `registrar/tests/integration_test.rs`
- `anoncreds/tests/integration_anoncreds.rs`
- `sdk/tests/integration_anoncreds.rs`
- `sdk/tests/reexports.rs`

## 7. Current Boundaries

The following are not yet implemented as first-class Rust workspace features:

- Vault-backed signer implementation.
- Generic lifecycle engine package equivalent to JS `lifecycle`.
- Separate `publisher-internal` crate/surface (publishing is currently handled directly through HCS operations in existing crates).
