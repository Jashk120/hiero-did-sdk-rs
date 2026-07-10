# hiero-did-sdk-rs

Rust workspace for creating, updating, deactivating, and resolving `did:hedera` identifiers, with reusable Hedera client and HCS service layers.

## Workspace Crates

- `hiero-did-core`: canonical DID types, document models, representation negotiation (`Accept`/`RepresentedDocument`), errors, and key utilities.
- `hiero-did-method`: parser/validator helpers for `did:hedera` and topic IDs.
- `hiero-did-messages`: signed envelope + DID event message models (owner/update/deactivate).
- `hiero-did-signer`: internal Ed25519 sign/verify helpers, plus optional HashiCorp Vault transit signing behind the `vault` feature.
- `hiero-did-client`: configurable Hedera client service for single or multi-network setups.
- `hiero-did-hcs`: topic/message/file helpers and higher-level HCS service with optional cache, signer-backed submit/admin keys, and local-node support.
- `hiero-did-registrar`: DID write operations (`create_did`, `update_did`, `deactivate_did`), signer-backed variants, and client-side message signing (CSM) prepare/submit flows.
- `hiero-did-resolver`: pluggable `TopicReader` abstraction with REST (`MirrorNodeClient`), gRPC (`GrpcTopicReader`), and cached (`HcsTopicReader`) backends. DID document reconstruction, DID URL dereference, and representation negotiation (JSON, JSON-LD, CBOR).
- `hiero-did-anoncreds`: AnonCreds registry layer on top of HCS.
- `hiero-did-lifecycle`: generic labeled lifecycle runner for DID operation orchestration, pause/resume boundaries, signing, and externally attached signatures.
- `hiero-did-utils`: shared test harness and polling helpers used by integration tests.
- `hiero-did-sdk`: umbrella crate that re-exports the workspace crates.

## Documentation

- Docs index: [`docs/README.md`](docs/README.md)
- API reference: [`docs/api-reference.md`](docs/api-reference.md)
- Create guide: [`docs/create-did.md`](docs/create-did.md)
- Resolve guide: [`docs/resolve-did.md`](docs/resolve-did.md)
- Dereference guide: [`docs/dereference-did.md`](docs/dereference-did.md)
- CSM guide: [`docs/csm.md`](docs/csm.md)
- Testing guide: [`docs/testing.md`](docs/testing.md)
- Architecture notes: [`ARCHITECTURE.md`](ARCHITECTURE.md)

## Prerequisites

- Rust + Cargo (workspace is pinned to `stable` via [`rust-toolchain.toml`](rust-toolchain.toml))
- Outbound network access for integration tests
- Hedera test account credentials for integration tests

Optional env file in repo root (`.env.local` preferred, `.env` also supported):

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
HEDERA_NETWORK=testnet
```

Notes:

- `HEDERA_PRIVATE_KEY` format must match the parser used by the specific integration test.
- For local-node tests, set `HEDERA_NETWORK=local` (or `local-node` / `localhost`).

## Build and Checks

```bash
cargo build --workspace
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

Check the Vault-backed signer feature:

```bash
cargo check -p hiero-did-signer --features vault
cargo test -p hiero-did-signer --features vault
```

## Tests

Run all tests:

```bash
cargo test --workspace
```

Run selected integration suites:

```bash
cargo test -p hiero-did-client --test client_service_integration -- --nocapture
cargo test -p hiero-did-hcs --test integration_hcs -- --nocapture
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
cargo test -p hiero-did-registrar --test csm_integration -- --ignored --nocapture
cargo test -p hiero-did-resolver --test grpc_integration -- --nocapture
cargo test -p hiero-did-resolver --test cbor_integration -- --nocapture
cargo test -p hiero-did-anoncreds --test integration_anoncreds -- --nocapture
cargo test -p hiero-did-sdk --test integration_anoncreds -- --nocapture
```

## Quick Start (Create + Resolve)

```rust
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_did_resolver::{resolve_did, MirrorNodeClient};
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let account_id = AccountId::from_str("0.0.12345")?;
    let operator_key = PrivateKey::from_str_der("<DER_PRIVATE_KEY>")?;

    let client = Client::for_testnet();
    client.set_operator(account_id, operator_key);

    // Create
    let created = create_did(&client, Network::Testnet, None).await?;
    println!("Created DID: {}", created.did);

    // Wait for mirror-node indexing, then resolve
    let mirror = MirrorNodeClient::for_testnet();
    mirror.wait_for_mirror(&created.did.topic_id, 30).await?;

    let resolution = resolve_did(&created.did.to_string(), None).await?;
    println!("Resolved DID: {}", resolution.did_document.id);
    Ok(())
}
```

## Pluggable Resolution Backends

The resolver decouples transport from resolution logic through the `TopicReader` trait. Three implementations are provided:

| Reader | Transport | Use case |
|---|---|---|
| `MirrorNodeClient` | REST (mirror-node HTTP API) | Default. Works in most environments. |
| `GrpcTopicReader` | gRPC (Hedera mirror subscription) | When REST is unavailable or you want stream semantics. |
| `HcsTopicReader` | gRPC via `HederaHcsService` | Reuse an existing `HederaHcsService` and its in-process cache. |

```rust
use hiero_did_resolver::{resolve_did, GrpcTopicReader};

// Use the default MirrorNodeClient (auto-selected from DID network):
let res = resolve_did("did:hedera:testnet:<key>_0.0.123", None).await?;

// Or pass a custom reader:
let grpc = GrpcTopicReader::for_testnet();
let res = resolve_did("did:hedera:testnet:<key>_0.0.123", Some(&grpc)).await?;
```

See [`docs/resolve-did.md`](docs/resolve-did.md) for the full guide.

## Using the Umbrella Crate

```rust
use hiero_did_sdk::{
    anoncreds, client, core, hcs, lifecycle, messages, method, registrar, resolver, signer,
};
```

## External Signers

The core signing abstraction is `hiero_did_core::Signer`. The registrar exposes signer-backed variants for DID operations:

- `create_did_with_signer`
- `update_did_with_signer`
- `deactivate_did_with_signer`

These accept any implementation of `Signer`, including `InternalSigner` and, when enabled, `VaultSigner`.

Enable Vault signing in `hiero-did-signer`:

```toml
hiero-did-signer = { path = "signer", features = ["vault"] }
```

Example Vault signer setup:

```rust
use hiero_did_signer::{VaultAuth, VaultSigner, VaultSignerConfig};

let cfg = VaultSignerConfig::new(
    "http://127.0.0.1:8200",
    VaultAuth::Token("vault-token".to_string()),
    "did-key",
);
let signer = VaultSigner::new(cfg)?;
```

For access-controlled HCS topics, `hiero-did-hcs` accepts `Arc<dyn Signer>` submit/admin signers. Signer failures are returned as `DIDError` instead of being converted to empty signatures.

## Client-Side Message Signing

CSM is for clients that cannot give the SDK a signer. The registrar prepares exact message bytes and serializable operation state, the client signs those bytes externally, and the registrar validates and submits the signed envelope.

Supported CSM APIs:

- `prepare_create_did_csm` / `submit_create_did_csm`
- `prepare_update_did_csm` / `submit_update_did_csm`
- `prepare_deactivate_did_csm` / `submit_deactivate_did_csm`
- `_with_options` prepare variants with optional expiry timestamps

CSM submit validates state version, deterministic request ID, exact rebuilt message bytes, optional expiry, Ed25519 signature length, and signature validity against the expected public key before writing to HCS.

See [`docs/csm.md`](docs/csm.md) for examples.

## Representation Negotiation

Resolved DID documents can be rendered in multiple formats via `represent()`:

- `Accept::DidJson` — DID document as JSON
- `Accept::DidLdJson` — DID document with `@context` as JSON-LD
- `Accept::DidResolution` — full resolution envelope (document + metadata)
- `Accept::DidCbor` — DID document as CBOR bytes (compact binary)

See [`docs/resolve-did.md`](docs/resolve-did.md) for usage.

## Current Boundaries

- Vault-backed signing is feature-gated and uses blocking HTTP internally to fit the synchronous `Signer` trait.
- Live Vault and live Hedera integration tests require external services and credentials.
- CSM live integration coverage is present but ignored by default because it requires Hedera credentials and mirror-node visibility.
