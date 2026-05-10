# Architecture: `hiero-did-sdk-rs`

## 1. Purpose and Scope

This workspace implements a Rust SDK for `did:hedera` lifecycle operations on Hedera, with a current focus on:

- DID creation by publishing signed owner events to Hedera Consensus Service (HCS).
- DID resolution by reconstructing a DID Document from mirror-node topic history.
- Reusable primitives for DID parsing, key encoding, message modeling, signing, and verification.

The architecture is intentionally split into small crates so applications can depend on only the layers they need.

## 2. Workspace Topology

The workspace is defined in `Cargo.toml` and contains these crates:

- `hiero-did-core`: canonical types and shared errors.
- `hiero-did-method`: DID parser and validators.
- `hiero-did-messages`: HCS envelope + DID event payload model.
- `hiero-did-signer`: Ed25519 signing and signature verification.
- `hiero-did-hcs`: Hedera client/topic helper operations.
- `hiero-did-registrar`: orchestration to create a DID and write initial event.
- `hiero-did-resolver`: mirror-node reader + DID document reconstruction.
- `hiero-did-sdk`: umbrella crate that re-exports all above.

Dependency direction is mostly one-way from high-level orchestration crates down to foundational crates:

```text
hiero-did-sdk (re-export only)
        |
        +--> registrar -----> hcs
        |         |           |
        |         +--> messages
        |         +--> signer
        |         +--> core
        |
        +--> resolver ------> messages
        |         |           |
        |         +--> signer
        |         +--> core
        |
        +--> method --------> core
        +--> signer --------> core
        +--> messages ------> core
        +--> hcs -----------> core
```

## 3. Crate Responsibilities in Detail

### 3.1 `hiero-did-core`

Primary responsibility: canonical data model and shared semantics.

Key modules:

- `did.rs`
  - `Network`: `Mainnet | Testnet` with `Display`/`FromStr`.
  - `HederaDid`: parsed DID struct with fields:
    - `network`
    - `base58_key`
    - `topic_id`
  - DID format encoded/parsed as:
    - `did:hedera:<network>:<base58key>_<shard>.<realm>.<topicNum>`
  - Constants:
    - `DID_METHOD = "hedera"`
    - `DID_ROOT_KEY_ID = "#did-root-key"`
- `document.rs`
  - DID Document and DID Resolution structs, including:
    - `VerificationMethod` (base58 or multibase variant)
    - `Service`
    - capability relationships (`authentication`, `assertionMethod`, etc.)
- `keys.rs`
  - `KeysUtility` for key conversions:
    - raw bytes <-> base58
    - raw bytes <-> multibase base58btc (`z...`) with Ed25519 multicodec prefix (`0xed, 0x01`)
- `error.rs`
  - shared `DIDError` enum with variants for validation, signature, not-found, serialization, and internal failures.

Architectural role:

- All other crates use `core` to avoid type duplication and ensure consistent DID parsing/formatting and error surfaces.

### 3.2 `hiero-did-method`

Primary responsibility: lightweight method-specific parsing/validation helpers.

Exports:

- `parse_did(&str) -> Result<HederaDid, DIDError>`
- `is_hedera_did(&str) -> bool`
- `is_topic_id(&str) -> bool`

Architectural role:

- Thin convenience layer over `core::HederaDid::FromStr`.
- Useful when consumers want boolean validation paths without full orchestration crates.

### 3.3 `hiero-did-messages`

Primary responsibility: protocol message shapes used for HCS transport.

Key structs:

- `HcsMessage`
  - fields: `timestamp`, `operation`, `did`, `event`.
  - this object is the signed payload (canonicalized by `serde_json::to_vec` in current implementation).
- `HcsEnvelope`
  - fields: `message: HcsMessage`, `signature` (base64).
  - this object is what gets written to HCS.
- Event model (`events.rs`)
  - `DIDEvent` enum (currently only `Owner` variant).
  - `DIDOwnerEvent` wraps `DIDOwnerEventData` under `"DIDOwner"` key.
  - Owner event includes:
    - `id`
    - `type` (currently `Ed25519VerificationKey2020`)
    - `controller`
    - `publicKeyMultibase`

Builder/orchestration helper:

- `DIDOwnerMessage`
  - Constructs a timestamped owner event.
  - Produces:
    - signable bytes (`message_bytes`)
    - signed envelope JSON (`to_payload(signature)`)

Architectural role:

- Defines wire contracts between registrar (writer) and resolver (reader).
- Encapsulates event serialization and envelope structure so other crates don’t duplicate encoding logic.

### 3.4 `hiero-did-signer`

Primary responsibility: local Ed25519 operations.

Types:

- `InternalSigner`
  - construct from 32-byte private key.
  - `sign(&[u8]) -> Vec<u8>`.
  - expose public key bytes from signing key.
- `InternalVerifier`
  - construct from 32-byte public key.
  - `verify(message, signature) -> Result<bool, DIDError>` for 64-byte signatures.

Architectural role:

- Provides a minimal crypto boundary for signing on create flow and verification on resolve flow.
- Keeps `ed25519-dalek` usage localized.

### 3.5 `hiero-did-hcs`

Primary responsibility: Hedera client wiring and topic operations.

`client.rs`:

- `HcsClient` wrapper around `hiero_sdk::Client` with testnet/mainnet constructors and operator setup.

`topic.rs`:

- `HcsTopic::create(client)`
- `HcsTopic::create_with_memo(client, memo)`
- `HcsTopic::submit(client, topic_id, message)`

Architectural role:

- Encapsulates direct Hedera transaction calls and maps errors into `DIDError::InternalError`.

### 3.6 `hiero-did-registrar`

Primary responsibility: end-to-end DID creation pipeline.

Entry point:

- `create::create_did(client, network, controller)`

Output:

- `CreateDIDResult`
  - `did`
  - `private_key_bytes` (sensitive)
  - `public_key_bytes`

Pipeline implemented in `create.rs`:

1. Generate Ed25519 keypair with `hiero_sdk::PrivateKey::generate_ed25519`.
2. Create HCS topic (`HcsTopic::create`).
3. Build DID using network + base58(public key) + topic ID.
4. Build `DIDOwnerMessage`.
5. Sign message bytes with `InternalSigner`.
6. Build envelope JSON payload with base64 signature.
7. Submit payload to topic.

Architectural role:

- Single high-level operation that coordinates core + messages + signer + HCS crates.
- Intended to be the default create API for most consumers.

### 3.7 `hiero-did-resolver`

Primary responsibility: read mirror-node topic history and reconstruct DID resolution result.

`mirror.rs`:

- `MirrorNodeClient` with network constructors:
  - `for_testnet()` -> `https://testnet.mirrornode.hedera.com`
  - `for_mainnet()` -> `https://mainnet.mirrornode.hedera.com`
- `get_topic_messages(topic_id)`:
  - fetches paginated `/api/v1/topics/{topic}/messages?order=asc&limit=100`
  - decodes base64 message payloads to UTF-8 JSON strings.

`builder.rs`:

- `DidDocumentBuilder::from(messages)`
- `resolve(&HederaDid) -> DIDResolution`

Resolver algorithm (current behavior):

1. Iterate topic messages in order.
2. Parse each into `HcsEnvelope`; skip malformed entries.
3. Filter by target DID string.
4. For `create` owner messages:
  - decode event payload.
  - parse `DIDEvent::Owner`.
  - derive verifier public key from `publicKeyMultibase`.
  - verify signature over serialized `HcsMessage`.
  - if valid, apply event to in-memory document state.
5. For `delete` messages:
  - verify signature with current verifier.
  - if valid, mark document deactivated and stop.
6. If no valid owner event applied, return `DIDError::NotFound`.
7. Build final DID Document and metadata.

Architectural role:

- Stateless event-folding engine over append-only HCS message history.
- Handles data integrity by per-message signature verification.

### 3.8 `hiero-did-sdk`

Primary responsibility: convenience import surface.

- Re-exports all crates under stable module names:
  - `core`, `method`, `messages`, `signer`, `hcs`, `registrar`, `resolver`.

Architectural role:

- Gives applications one dependency option without losing modular internals.

## 4. End-to-End Runtime Flows

### 4.1 DID Creation Flow

```text
Caller
  -> registrar::create_did
    -> generate local Ed25519 keypair
    -> hcs::HcsTopic::create
    -> core::HederaDid::new
    -> messages::DIDOwnerMessage::message_bytes
    -> signer::InternalSigner::sign
    -> messages::DIDOwnerMessage::to_payload
    -> hcs::HcsTopic::submit
  <- CreateDIDResult { did, private_key_bytes, public_key_bytes }
```

Key properties:

- Private key is generated locally and returned to caller.
- Topic ID is a first-class DID component.
- Initial owner event anchors the root verification method.

### 4.2 DID Resolution Flow

```text
Caller
  -> resolver::MirrorNodeClient::get_topic_messages(topic_id)
    -> mirror-node pagination fetch
    -> base64 decode to JSON strings
  -> resolver::DidDocumentBuilder::from(messages).resolve(did)
    -> envelope parse + did filter
    -> event decode + signature verification
    -> state fold (owner/create, optional delete)
  <- DIDResolution { didDocument, didDocumentMetadata, didResolutionMetadata }
```

Key properties:

- Resolution is deterministic for a given ordered message list.
- Invalid/malformed/failed-signature entries are ignored.
- Not-found is explicit if no valid owner event exists.

## 5. Data Contracts and Encoding

### 5.1 DID String Contract

- Format: `did:hedera:<network>:<base58pubkey>_<topicId>`
- `topicId` is expected in `shard.realm.num` numeric form.
- Root key fragment is always `#did-root-key`.

### 5.2 Envelope Contract

Transport payload written to HCS is JSON of:

- `message` (object)
- `signature` (base64 string)

Where `message.event` is itself base64 of event JSON.

### 5.3 Key Encoding Rules

- DID string key component uses base58 of raw Ed25519 public key bytes.
- Event `publicKeyMultibase` uses multibase base58btc with multicodec Ed25519 prefix.

## 6. Trust and Security Boundaries

### 6.1 Local Trust Boundary

Trusted local operations:

- key generation
- message serialization
- signing

Sensitive output:

- `CreateDIDResult.private_key_bytes` must be treated as secret material.

### 6.2 Network Trust Boundary

Untrusted external inputs:

- mirror node responses
- topic message bodies

Defensive controls present:

- parse guards (skip malformed)
- base64/UTF-8 decode checks
- signature verification before state application
- DID filter to ignore unrelated topic entries

### 6.3 Consistency Model

- HCS submit is immediate from client perspective, but mirror-node visibility is eventually consistent.
- Integration tests explicitly wait before resolution.

## 7. Error Model

All crates converge on `core::DIDError`.

Common mappings:

- validation/parsing issues -> `InvalidDid`, `InvalidArgument`, `InvalidMultibase`
- signature shape/content issues -> `InvalidSignature`
- no resolvable document -> `NotFound`
- external/network/SDK failures -> `InternalError`
- serde failures -> `SerializationError`

Architectural impact:

- Consumers can handle one error enum across all SDK layers.

## 8. Testing Architecture

### 8.1 Unit Tests

Present in foundational crates (`core`, `method`, `signer`, `resolver::builder`) to validate:

- DID parse/display round-trips
- key encoding round-trips
- signature correctness and failure paths
- resolver behavior for create, tampered signature, delete, and unrelated DID filtering

### 8.2 Integration Tests

`registrar/tests/integration_test.rs` performs networked e2e tests:

- create DID against Hedera testnet.
- create + resolve DID via mirror node.

Inputs required via `.env`:

- `HEDERA_ACCOUNT_ID`
- `HEDERA_PRIVATE_KEY` (DER format)

## 9. Design Choices and Tradeoffs

### 9.1 Strengths

- Clean crate boundaries with low coupling.
- Reusable foundational types and consistent errors.
- Event-sourced resolution model suitable for append-only HCS history.
- Signature-verified application of state transitions.

### 9.2 Current Constraints

- Event model currently supports owner events only (`DIDOwner`); no service/key-agreement update events yet.
- Resolver currently tracks verification methods and basic auth/assertion defaults; service/key-agreement/delegation updates are not yet event-driven.
- Signature verification uses message JSON serialization at verification time; this depends on stable serialization semantics matching signing phase.
- `hcs::HcsClient` wrapper exists but `registrar` currently accepts raw `hiero_sdk::Client` directly.

## 10. Extension Points

Natural extension paths consistent with current architecture:

1. Add new `DIDEvent` variants (`Service`, `VerificationMethod`, relationship updates, etc.) in `messages`.
2. Extend resolver fold logic with operation/event handlers per variant.
3. Add registrar/update APIs for append operations beyond create/delete.
4. Introduce canonical message normalization rules if cross-language interoperability needs stricter deterministic signing inputs.
5. Expand mirror client options (timeouts/retries/backoff/configurable endpoints).
6. Add key custody abstractions so callers can integrate HSM/KMS instead of raw key bytes.

## 11. File Map (Implementation Anchors)

Core architecture files:

- `core/src/did.rs`
- `core/src/document.rs`
- `core/src/keys.rs`
- `core/src/error.rs`
- `messages/src/envelope.rs`
- `messages/src/events.rs`
- `messages/src/did_owner.rs`
- `signer/src/lib.rs`
- `hcs/src/topic.rs`
- `resolver/src/mirror.rs`
- `resolver/src/builder.rs`
- `registrar/src/create.rs`
- `sdk/src/lib.rs`

## 12. Practical Mental Model

Think of the SDK as three layers:

1. Foundation layer (`core`, `method`, `signer`, `messages`): types, encoding, signing, validation.
2. Infrastructure layer (`hcs`, `resolver::mirror`): write/read transport with Hedera and mirror node.
3. Workflow layer (`registrar`, `resolver::builder`): business flow to create and resolve DIDs.

That layering keeps protocol details explicit while still offering high-level entry points for application teams.
