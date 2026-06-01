# Architecture: `hiero-did-sdk-rs`

## 1. Scope

This Rust workspace currently provides:

- DID write operations: create, update, deactivate (`registrar`).
- Client-side message signing prepare/submit flows for DID writes (`registrar::csm`).
- DID resolution from topic history (`resolver`).
- DID URL dereference for resolved documents/resources (`resolver` + `core::did_url`).
- Hedera client configuration (`client`) and HCS topic/message/file operations (`hcs`).
- Shared DID/domain primitives (`core`, `method`, `messages`, `signer`).
- Generic lifecycle orchestration primitives (`lifecycle`).
- AnonCreds registry operations on top of HCS (`anoncreds`).
- A convenience re-export layer (`sdk`).
- A local scratch binary crate for experiments (`scratch`).

Toolchain baseline:

- Workspace is pinned to Rust `nightly` in `rust-toolchain.toml` (`rustfmt` and `clippy` components included).

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
- `hiero-did-lifecycle`
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
        |         +--> lifecycle
        |         +------------+
        |
        +--> resolver ------> messages -----> core
        |         |
        |         +--> signer
        |
        +--> anoncreds -----> hcs ----------> client -----> core
        |
        +--> lifecycle -----> core
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

Ed25519 signing and verification boundary:

- `InternalSigner` for private-key signing.
- `InternalVerifier` for signature validation.
- Optional `vault` feature:
  - `VaultSigner` for HashiCorp Vault transit-backed Ed25519 signing.
  - `VaultSignerConfig` and `VaultAuth` for Vault URL, key, mount path, token, and AppRole configuration.
  - Vault signing keeps private key material outside the SDK and returns raw 64-byte Ed25519 signatures through the shared `core::Signer` trait.

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
- Shared transaction-signing helper (`hcs::signing`) adapts `core::Signer` instances to Hedera transaction `sign_with` callbacks and preserves signer errors instead of replacing failures with empty signatures.

### 3.7 `hiero-did-registrar`

DID write orchestration:

- `create::create_did(client, network, controller)`
- `create::create_did_with_signer(client, network, controller, signer)`
- `update::update_did(client, did, private_key_bytes, updates)`
- `update::update_did_with_signer(client, did, signer, updates)`
- `deactivate::deactivate_did(client, did, private_key_bytes)`
- `deactivate::deactivate_did_with_signer(client, did, signer)`
- `csm::prepare_create_did_csm(...)` / `csm::submit_create_did_csm(...)`
- `csm::prepare_update_did_csm(...)` / `csm::submit_update_did_csm(...)`
- `csm::prepare_deactivate_did_csm(...)` / `csm::submit_deactivate_did_csm(...)`
- `_with_options` CSM prepare variants support optional expiry timestamps.

The update path supports verification method and service add/remove operations via `DIDUpdateOperation`.
The `*_with_signer` APIs accept any `core::Signer`, including `InternalSigner` and feature-gated `VaultSigner`.
The CSM APIs prepare exact message bytes and serializable operation state for external signing, then validate and submit externally signed envelopes.

### 3.8 `hiero-did-lifecycle`

Generic linear lifecycle runner:

- `LifecycleBuilder` defines labeled callback, signer-sign, attach-signature, and pause steps.
- `LifecycleRunner` executes the pipeline and resumes from pause states.
- `RunnerState` carries `Success`, `Pause`, or `Error` status plus the mutable message.
- `LifecycleMessage` abstracts operation messages that can expose bytes to sign and accept a signature.

The crate is DID-domain neutral. Registrar CSM currently uses lifecycle-compatible labels and state boundaries; concrete registrar flows still own Hedera message construction and submission.

### 3.9 `hiero-did-resolver`

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

### 3.10 `hiero-did-anoncreds`

AnonCreds registry layer on top of HCS service:

- `HederaAnonCredsRegistry` operations for:
- schema register/get
- credential definition register/get
- revocation registry definition register/get
- revocation status list register/get
- `types` module for AnonCreds payload models and HCS metadata.
- `utils` module for identifier building/parsing and revocation entry pack/unpack/diff helpers.

### 3.11 `hiero-did-sdk`

Convenience import surface by re-exporting:

- `core`, `method`, `messages`, `signer`, `client`, `hcs`, `registrar`, `resolver`, `anoncreds`, `lifecycle`.

### 3.12 `scratch`

Local binary crate used for ad-hoc experiments:

- Not re-exported by `hiero-did-sdk`.
- Not part of the public SDK contract.
- Can change independently without semantic compatibility guarantees.

## 4. Runtime Flows

### 4.1 Create DID

Default local-key flow:

1. Generate Ed25519 keypair.
2. Create HCS topic.
3. Build DID: `did:hedera:<network>:<base58key>_<topicId>`.
4. Build owner message and sign serialized `HcsMessage` bytes.
5. Submit envelope to topic.
6. Return DID and raw key bytes.

External signer flow:

1. Read public key bytes from the provided `core::Signer`.
2. Create HCS topic.
3. Build DID from the signer public key and topic ID.
4. Build owner message and sign serialized message bytes through `Signer::sign_bytes`.
5. Submit envelope to topic.
6. Return DID and public key bytes. Private key bytes are not returned because key custody is external.

### 4.2 Update DID

1. Parse topic ID from target DID.
2. Convert each `DIDUpdateOperation` to an update message.
3. Sign serialized message bytes with either DID private key bytes (`update_did`) or an external `core::Signer` (`update_did_with_signer`).
4. Submit each signed envelope to the same DID topic in order.
5. Return applied operation count.

Notes:

- Empty update list returns early with `operations_applied = 0`.
- `verificationMethod` updates require `public_key_multibase`.

### 4.3 Deactivate DID

1. Parse topic ID from target DID.
2. Build deactivate message (`operation = "delete"`).
3. Sign serialized message bytes with either DID private key bytes (`deactivate_did`) or an external `core::Signer` (`deactivate_did_with_signer`).
4. Submit signed envelope to DID topic.
5. Return tombstoned document metadata payload.

### 4.4 Client-Side Message Signing

CSM is used when the SDK must not call a signer directly.

Prepare flow:

1. Build the create/update/deactivate DID message.
2. Compute the exact `message_bytes` to sign.
3. Build `CsmOperationState` with:
- state version
- deterministic request ID
- DID, topic ID, operation, and lifecycle label
- optional expiry timestamp
- expected public key bytes
- exact message bytes and message state
4. Return `CsmSigningRequest` or `CsmBatchSigningRequest`.

External client flow:

1. Sign each `message_bytes` value outside the SDK.
2. Convert the request into `CsmSubmitRequest` or `CsmBatchSubmitRequest`.

Submit flow:

1. Validate state version and deterministic request ID.
2. Rebuild message bytes and compare against preserved bytes.
3. Reject expired requests when `expires_at_unix` is set.
4. Verify the 64-byte Ed25519 signature against the expected public key.
5. Build the signed `HcsEnvelope`.
6. Submit to the DID topic.

Create CSM creates the HCS topic during prepare because the topic ID is part of the DID and owner message. Update CSM returns one signing request per update operation and submits signed operations in order.

### 4.5 Resolve DID

1. Fetch topic message history from mirror node.
2. Decode envelopes and filter by target DID.
3. Establish verifier from DID owner event key material.
4. Verify signatures against that key.
5. Apply owner/update/service events; treat `operation == "delete"` as deactivation.
6. Emit `DIDResolution` with metadata.

### 4.6 HCS Service Usage

1. Build `HederaClientConfiguration`.
2. Create `HederaClientService`.
3. Optionally attach `HcsCacheService`.
4. Use `HederaHcsService` for topic/message/file operations by network name.
5. For access-controlled topics, pass `Arc<dyn Signer>` submit/admin signers. HCS captures signer failures and returns the original `DIDError`.

### 4.7 Dereference DID URL

1. Parse DID URL into `HederaDidUrl`.
2. Obtain topic messages for the DID topic (typically via `MirrorNodeClient`).
3. Call `dereference_did(&did_url, messages)`.
4. If URL has no fragment, return full `DIDDocument`.
5. If URL has a fragment, match `did#fragment` against verification methods and services.

### 4.8 AnonCreds Registry Usage

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
- `registrar/tests/csm_integration.rs` (ignored by default; live Hedera + mirror-node flow)
- `anoncreds/tests/integration_anoncreds.rs`
- `sdk/tests/integration_anoncreds.rs`
- `sdk/tests/reexports.rs`

## 7. Current Boundaries

The following are not yet implemented as first-class Rust workspace features:

- Separate `publisher-internal` crate/surface (publishing is currently handled directly through HCS operations in existing crates).

Current caveats:

- Vault signing is feature-gated in `hiero-did-signer` behind the `vault` feature.
- Vault HTTP calls are currently implemented with `reqwest::blocking` to satisfy the synchronous `core::Signer` trait.
- Live Vault, live Hedera, and CSM end-to-end integration coverage still require external services, credentials, and mirror-node visibility.
