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
- `did`: `Network` (Mainnet, Testnet, Previewnet, Local), `HederaDid`, `DID_METHOD`, `DID_ROOT_KEY_ID`.
- `did_url`: `HederaDidUrl` parser for DID URLs.
- `document`: DID document, verification method, service, and resolution metadata models.
- `keys`: `KeysUtility` for base58 and multibase conversion.
- `error`: shared `DIDError` enumeration.
- `signer`: crate-agnostic synchronous `Signer` trait.

### 3.2 `hiero-did-method`
Parser and validation helpers:
- `parse_did`: helper for `HederaDid::from_str`.
- `is_hedera_did`: validation check.
- `is_topic_id`: format check (shard.realm.num).

### 3.3 `hiero-did-messages`
Wire-level message models used in HCS payloads:
- `HcsMessage`: inner signed message.
- `HcsEnvelope`: outer signed envelope.
- `DIDOwnerMessage`: owner/create message.
- `DIDUpdateOperation` messages: add/remove verification methods and services.
- `DIDDeactivateMessage`: delete operation message.
- Event models: typed payload views for owner, verification method, service, and deactivation events.

### 3.4 `hiero-did-signer`
Ed25519 signing and verification boundary:
- `InternalSigner`: in-process private-key signing.
- `InternalVerifier`: signature validation.
- Optional `vault` feature:
  - `VaultSigner`: adapter for HashiCorp Vault transit signing.
  - `VaultSignerConfig`: configuration for Vault server, auth, and key name.
  - `VaultAuth`: `Token` or `AppRole` authentication methods.

The Vault-backed signer keeps private key material outside the SDK and adapts Vault's HTTP API to the shared `core::Signer` trait.

### 3.5 `hiero-did-client`
Configurable network-aware Hedera client factory:
- `HederaClientConfiguration`: collection of network configs.
- `NetworkConfig`: network, operator ID, and operator key pairing.
- `HederaCustomNetwork`: explicit node address and mirror-node lists.
- `HederaNetwork`: enum for `Mainnet`, `Testnet`, `Previewnet`, `LocalNode`, or `Custom`.
- `HederaClientService`: manages client lifecycle and lookup by network name.
- `NetworkName`: descriptor for selecting a specific configured network.

### 3.6 `hiero-did-hcs`
HCS primitives and service facade:
- `HcsTopic`: create/update/delete/info/submit topic operations.
- `HcsMessage`: submit and fetch topic messages with mirror-node subscription support.
- `HcsFileService`: chunked and compressed (zstd) payload publish/resolve over HCS-1 style messaging.
- `HcsCacheService`: in-memory (moka) cache for topic info, messages, and files.
- `HederaHcsService`: facade combining client service with optional caching.
- Shared transaction-signing adapter for `Arc<dyn core::Signer>` submit/admin keys.

### 3.7 `hiero-did-registrar`
DID write orchestration:
- `create::create_did` / `create_did_with_signer` (local key vs external signer).
- `update::update_did` / `update_did_with_signer` (batch operation support).
- `deactivate::deactivate_did` / `deactivate_did_with_signer`.
- `csm`: client-side message signing (CSM) prepare/submit flows for all operations.
- `CsmBatchSigningRequest` / `CsmBatchSubmitRequest`: support for signing multiple operations.

The update path supports verification method (VerificationMethod, Authentication, etc.) and service add/remove operations.

### 3.8 `hiero-did-lifecycle`
Generic linear lifecycle runner:
- `LifecycleBuilder`: fluent definition of labeled pipeline steps.
- `LifecycleRunner`: executes the pipeline with support for pause-for-external-signature.
- `LifecycleMessage`: abstraction for messages requiring signatures.
- `RunnerState` / `RunnerStatus`: tracking progress across async boundaries.

### 3.9 `hiero-did-resolver`
Resolution orchestration:
- `MirrorNodeClient`: fetches topic messages from mirror-node REST APIs.
- `DidDocumentBuilder`: folds validated events into a DID document.
- `dereference_did`: resolves DID URLs against document resources (verification systems, services).
- `DereferencedResource`: represents documents, verification methods, or services.

### 3.10 `hiero-did-anoncreds`
AnonCreds registry layer on top of `HederaHcsService`:
- `HederaAnonCredsRegistry`: main registration and lookup API.
- Management of Schemas, Credential Definitions, and Revocation Registry Definitions as HCS files.
- Revocation Status List management via topic message diffs.
- `types` / `utils`: AnonCreds-specific serialization models and revocation entry helpers.

### 3.11 `hiero-did-utils`
Workspace support crate:
- `utils::tests`: shared test harness and setup utilities.
- Used by integration tests for environment-aware configurations.

### 3.12 `hiero-did-sdk`
Umbrella import surface that re-exports all public workspace crates (anoncreds, client, core, hcs, lifecycle, messages, method, registrar, resolver, signer).

### 3.13 `scratch`
Local binary crate for ad-hoc experiments. Not part of the public SDK surface or contract.

## 4. Runtime Flows

### 4.1 Create DID
1. Generate or provide public key material.
2. Create an HCS topic.
3. Build DID identifier.
4. Build owner message, sign, and submit signed envelope to topic.

### 4.2 Update DID
1. Parse topic ID from DID.
2. Convert `DIDUpdateOperation` list to sequence of signed messages.
3. Submit envelopes in order to the HCS topic.

### 4.3 Client-Side Message Signing (CSM)
1. **Prepare**: SDK builds message, calculates bytes, and captures state.
2. **External Sign**: Client signs bytes outside the SDK.
3. **Submit**: SDK validates state/signature and submits to HCS.

### 4.4 Resolve DID
1. Fetch topic message history via `MirrorNodeClient`.
2. Filter and validated events (owner -> updates -> deactivation).
3. fold events into final `DIDDocument`.

## 5. Error Model
Domain, network, and signing failures map to `hiero_did_core::DIDError`:
- `InvalidDid`: parsing/format failures.
- `InvalidArgument`: null inputs or out-of-range values.
- `InvalidSignature`: crypto validation failures.
- `NotFound`: missing topic/message/service.
- `InternalError`: network or system failures.
- `SerializationError`: JSON/CBOR failures.

## 6. Testing Strategy
- Unit tests: located within each crate.
- Integration tests: dedicated files in `tests/` directories using live Hedera or mirror-node mocks where possible.
- Coverage includes CSM flows, HCS file chunking, and AnonCreds registry logic.

## 7. Current Boundaries
- Vault signing: uses synchronous trait bounds via `reqwest::blocking`.
- persistence: No database-backed persistence layer; resolution is derived from HCS topic history.
- Smart Contracts: No on-chain smart contract execution; all logic is event-driven over HCS.
