# Architecture: `hiero-did-sdk-rs`

## 1. Scope

This repository is a Rust workspace for `did:hedera` operations on Hiero/Hedera. It provides:

- DID write operations: create, update, deactivate.
- Client-side message signing (CSM) prepare/submit flows for DID writes.
- DID resolution from HCS topic history through mirror-node reads.
- DID URL dereference for resolved documents, verification methods, and services.
- Hedera client configuration and HCS topic/message/file operations.
- Shared DID/domain primitives, message models, signing boundaries, and lifecycle orchestration.
- AnonCreds registry operations on top of HCS.
- An umbrella SDK crate that re-exports the public SDK crates.
- Local utility and scratch crates that support tests or experiments but are not part of the umbrella SDK surface.

Toolchain baseline:

- The workspace is pinned to Rust `nightly` in `rust-toolchain.toml`.
- `rustfmt` and `clippy` components are included by the toolchain file.
- Most crates use Rust 2024 edition; `hiero-did-anoncreds` currently uses Rust 2021 edition.

## 2. Workspace Topology

Workspace members in root `Cargo.toml`:

- `core` -> package `hiero-did-core`
- `method` -> package `hiero-did-method`
- `messages` -> package `hiero-did-messages`
- `signer` -> package `hiero-did-signer`
- `client` -> package `hiero-did-client`
- `hcs` -> package `hiero-did-hcs`
- `registrar` -> package `hiero-did-registrar`
- `resolver` -> package `hiero-did-resolver`
- `anoncreds` -> package `hiero-did-anoncreds`
- `lifecycle` -> package `hiero-did-lifecycle`
- `utils` -> package `hiero-did-utils`
- `sdk` -> package `hiero-did-sdk`
- `scratch` -> package `scratch`

Runtime dependency direction:

```text
hiero-did-sdk
  |-- hiero-did-anoncreds --> hiero-did-hcs --> hiero-did-client --> hiero-did-core
  |                         |                \------------------> hiero-did-core
  |                         \-----------------------------------> hiero-did-core
  |-- hiero-did-registrar --> hiero-did-hcs
  |                         --> hiero-did-messages --> hiero-did-core
  |                         --> hiero-did-signer ----> hiero-did-core
  |                         --> hiero-did-core
  |-- hiero-did-resolver ---> hiero-did-messages
  |                         --> hiero-did-signer
  |                         --> hiero-did-core
  |-- hiero-did-lifecycle --> hiero-did-core
  |-- hiero-did-method ----> hiero-did-core
  |-- hiero-did-client ----> hiero-did-core
  |-- hiero-did-hcs -------> hiero-did-core
  |-- hiero-did-messages --> hiero-did-core
  |-- hiero-did-signer ---> hiero-did-core
  \-- hiero-did-core

hiero-did-utils: test/support helpers; not re-exported by `hiero-did-sdk`.
scratch: local binary crate; not re-exported by `hiero-did-sdk`.
```

Important boundaries:

- `core` is the shared model/error/signing-trait base.
- `messages` owns serializable DID operation envelopes and event payloads, but does not publish to HCS.
- `hcs` owns Hedera Consensus Service primitives and service-level wrappers, but does not know DID event semantics.
- `registrar` owns DID write semantics and uses `hcs`, `messages`, and signer abstractions to publish DID operation events.
- `resolver` owns DID read semantics and folds message history into DID documents.
- `anoncreds` is a registry layer over `hcs`, not a dependency of DID create/update/resolve flows.
- `lifecycle` is generic orchestration over lifecycle-compatible messages; current registrar CSM code uses matching state/label concepts but does not depend on the lifecycle crate.

## 3. Crate Responsibilities

### 3.1 `hiero-did-core`

Canonical DID model and shared structures:

- `did`: `Network`, `HederaDid`, `DID_METHOD`, `DID_ROOT_KEY_ID`.
- `did_url`: `HederaDidUrl` parser for DID URLs.
- `document`: DID document, verification method, service, and resolution metadata models.
- `keys`: base58 and multibase conversion utilities.
- `error`: shared `DIDError`.
- `signer`: crate-agnostic synchronous `Signer` trait.

### 3.2 `hiero-did-method`

Parser and validation helpers:

- `parse_did`
- `is_hedera_did`
- `is_topic_id`

### 3.3 `hiero-did-messages`

Wire-level message models used in HCS payloads:

- `HcsMessage`
- `HcsEnvelope`
- `DIDOwnerMessage`
- `DIDAddVerificationMethodMessage`
- `DIDRemoveVerificationMethodMessage`
- `DIDAddServiceMessage`
- `DIDRemoveServiceMessage`
- `DIDDeactivateMessage`
- Event models for owner, verification method, service, and deactivate payloads.

### 3.4 `hiero-did-signer`

Ed25519 signing and verification boundary:

- `InternalSigner` for in-process private-key signing.
- `InternalVerifier` for signature validation.
- Optional `vault` feature:
  - `VaultSigner`
  - `VaultSignerConfig`
  - `VaultAuth`

The Vault-backed signer keeps private key material outside the SDK and adapts HashiCorp Vault transit signing to the shared `core::Signer` trait.

### 3.5 `hiero-did-client`

Configurable network-aware Hedera client factory:

- `HederaClientConfiguration`
- `NetworkConfig`
- `HederaCustomNetwork`
- `HederaNetwork`
- `HederaClientService`
- `NetworkName`

Supported network variants are `Mainnet`, `Testnet`, `Previewnet`, `LocalNode`, and `Custom`.

### 3.6 `hiero-did-hcs`

HCS primitives and service facade:

- `HcsTopic`: create/update/delete/info/submit topic operations.
- `HcsMessage`: submit and fetch topic messages.
- `HcsFileService`: chunked and compressed payload publish/resolve.
- `HcsCacheService`: topic info, message, and file cache.
- `HederaHcsService`: combines `HederaClientService` with optional cache.
- `HcsClient` and `LocalSigner`: lightweight client/signing helpers.
- Shared transaction-signing adapter for `Arc<dyn core::Signer>` submit/admin keys.

Signer-backed HCS operations preserve signer failures as `DIDError` instead of replacing failures with empty signatures.

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
- `_with_options` CSM prepare variants with optional expiry timestamps.
- Batch CSM submit support through `CsmBatchSigningRequest`, `CsmBatchSubmitRequest`, and `submit_csm_batch`.

The update path supports verification method and service add/remove operations via `DIDUpdateOperation`.

The `*_with_signer` APIs accept any `core::Signer`, including `InternalSigner` and feature-gated `VaultSigner`.

The CSM APIs prepare exact message bytes and serializable operation state for external signing, then validate and submit externally signed envelopes.

### 3.8 `hiero-did-lifecycle`

Generic linear lifecycle runner:

- `LifecycleBuilder` defines labeled callback, signer-sign, attach-signature, and pause steps.
- `LifecycleRunner` executes the pipeline and resumes from pause states.
- `LifecycleRunnerOptions` supplies signer and externally attached signature inputs.
- `RunnerState` and `RunnerStatus` describe success, pause, and error states.
- `LifecycleMessage` abstracts messages that expose bytes to sign and accept a signature.
- `LifecycleStep`, `LifecycleStepKind`, and `LifecycleFuture` define the step model.

The crate is DID-domain neutral and depends only on `hiero-did-core`.

### 3.9 `hiero-did-resolver`

Resolution orchestration:

- `MirrorNodeClient` fetches topic messages from mirror-node REST APIs.
- `DidDocumentBuilder` folds validated events into a DID document.
- `dereference::dereference_did` resolves DID URLs against topic message history.
- `DereferencedResource` represents whole documents, verification methods, or services.

The resolver verifies signatures before applying events and applies create, update, service, and deactivate semantics from the event stream.

### 3.10 `hiero-did-anoncreds`

AnonCreds registry layer on top of `HederaHcsService`:

- `HederaAnonCredsRegistry`
- Schema register/get operations.
- Credential definition register/get operations.
- Revocation registry definition register/get operations.
- Revocation status list register/get operations.
- `types` module for AnonCreds payload models and HCS metadata.
- `utils` module for identifier building/parsing and revocation entry pack/unpack/diff helpers.

### 3.11 `hiero-did-utils`

Workspace support crate:

- Currently exposes test/support utilities under `utils::tests`.
- Used by integration tests.
- Not re-exported by `hiero-did-sdk`.
- Not part of the public SDK compatibility surface.

### 3.12 `hiero-did-sdk`

Umbrella import surface that re-exports:

- `anoncreds`
- `client`
- `core`
- `hcs`
- `lifecycle`
- `messages`
- `method`
- `registrar`
- `resolver`
- `signer`

It intentionally does not re-export `utils` or `scratch`.

### 3.13 `scratch`

Local binary crate for ad-hoc experiments:

- Depends directly on `hiero-sdk` and `tokio`.
- Not re-exported by `hiero-did-sdk`.
- Not part of the public SDK contract.
- Can change independently without semantic compatibility guarantees.

## 4. Runtime Flows

### 4.1 Create DID

Default local-key flow:

1. Generate an Ed25519 keypair.
2. Create an HCS topic.
3. Build DID: `did:hedera:<network>:<base58key>_<topicId>`.
4. Build an owner message and sign serialized `HcsMessage` bytes.
5. Submit the signed envelope to the topic.
6. Return the DID and raw key bytes.

External signer flow:

1. Read public key bytes from the provided `core::Signer`.
2. Create an HCS topic.
3. Build the DID from signer public key and topic ID.
4. Build the owner message and sign serialized message bytes through `Signer::sign_bytes`.
5. Submit the signed envelope to the topic.
6. Return the DID and public key bytes. Private key bytes are not returned because key custody is external.

### 4.2 Update DID

1. Parse the topic ID from the target DID.
2. Convert each `DIDUpdateOperation` to an update message.
3. Sign serialized message bytes with either DID private key bytes or an external `core::Signer`.
4. Submit each signed envelope to the DID topic in order.
5. Return the applied operation count.

Notes:

- Empty update lists return early with `operations_applied = 0`.
- Verification method updates require `public_key_multibase`.

### 4.3 Deactivate DID

1. Parse the topic ID from the target DID.
2. Build a deactivate message with `operation = "delete"`.
3. Sign serialized message bytes with either DID private key bytes or an external `core::Signer`.
4. Submit the signed envelope to the DID topic.
5. Return tombstoned document metadata payload.

### 4.4 Client-Side Message Signing

CSM is used when the SDK must not call a signer directly.

Prepare flow:

1. Build the create, update, or deactivate DID message.
2. Compute the exact `message_bytes` to sign.
3. Build `CsmOperationState` with state version, deterministic request ID, DID/topic/operation metadata, lifecycle label, optional expiry, expected public key bytes, preserved message bytes, and message state.
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

1. Fetch topic message history from a mirror node.
2. Decode envelopes and filter by target DID.
3. Establish verifier key material from the DID owner event.
4. Verify signatures before applying events.
5. Apply owner/update/service events; treat `operation == "delete"` as deactivation.
6. Emit `DIDResolution` with document and metadata.

### 4.6 Dereference DID URL

1. Parse a DID URL into `HederaDidUrl`.
2. Obtain topic messages for the DID topic, typically through `MirrorNodeClient`.
3. Call `dereference_did(&did_url, messages)`.
4. If the URL has no fragment, return the full `DIDDocument`.
5. If the URL has a fragment, match `did#fragment` against verification methods and services.

### 4.7 HCS Service Usage

1. Build `HederaClientConfiguration`.
2. Create `HederaClientService`.
3. Optionally attach `HcsCacheService`.
4. Use `HederaHcsService` for topic, message, and file operations by network name.
5. For access-controlled topics, pass `Arc<dyn Signer>` submit/admin signers.

### 4.8 AnonCreds Registry Usage

1. Build `HederaClientService` and `HederaHcsService`.
2. Construct `HederaAnonCredsRegistry`.
3. Register/read AnonCreds objects via DID and topic-backed identifiers.

## 5. Error Model

Crates map domain, serialization, network, and signing failures to `hiero_did_core::DIDError` variants:

- `InvalidDid`
- `InvalidArgument`
- `InvalidSignature`
- `InvalidMultibase`
- `NotFound`
- `InternalError`
- `SerializationError`

## 6. Testing Strategy

Unit tests cover parsing, key conversion, signing, event application, lifecycle execution, HCS helpers, and registry utilities.

Integration tests currently include:

- `client/tests/client_service_integration.rs`
- `hcs/tests/integration_hcs.rs`
- `registrar/tests/integration_test.rs`
- `registrar/tests/csm_integration.rs` (ignored by default; live Hedera + mirror-node flow)
- `anoncreds/tests/integration_anoncreds.rs`
- `lifecycle/tests/lifecycle.rs`
- `sdk/tests/integration_anoncreds.rs`
- `sdk/tests/reexports.rs`

Live Hedera, mirror-node, and Vault-related coverage requires external services and credentials.

## 7. Current Boundaries

The following are not first-class workspace surfaces:

- Separate publisher crate. Publishing is currently handled through `hiero-did-hcs` and higher-level registrar/anoncreds flows.
- `hiero-did-utils` as public SDK API. It is test/support infrastructure.
- `scratch` as public SDK API. It is local experiment code.

Current caveats:

- Vault signing is feature-gated in `hiero-did-signer` behind the `vault` feature.
- Vault HTTP calls are implemented with `reqwest::blocking` to satisfy the synchronous `core::Signer` trait.
- Live Vault, live Hedera, and CSM end-to-end integration coverage require external services, credentials, and mirror-node visibility.
