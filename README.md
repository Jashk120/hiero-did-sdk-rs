# hiero-did-sdk-rs

Rust workspace for creating, updating, deactivating, and resolving `did:hedera` identifiers, with reusable Hedera client and HCS service layers.

## Workspace Crates

- `hiero-did-core`: canonical DID types, document models, errors, and key utilities.
- `hiero-did-method`: parser/validator helpers for `did:hedera` and topic IDs.
- `hiero-did-messages`: signed envelope + DID event message models (owner/update/deactivate).
- `hiero-did-signer`: internal Ed25519 sign/verify helpers, plus optional HashiCorp Vault transit signing behind the `vault` feature.
- `hiero-did-client`: configurable Hedera client service for single or multi-network setups.
- `hiero-did-hcs`: topic/message/file helpers and higher-level HCS service with optional cache and signer-backed submit/admin key support.
- `hiero-did-registrar`: DID write operations (`create_did`, `update_did`, `deactivate_did`) plus signer-backed variants for external key custody.
- `hiero-did-resolver`: mirror-node reader + DID document reconstruction + DID URL dereference helpers.
- `hiero-did-anoncreds`: AnonCreds registry layer on top of HCS.
- `hiero-did-sdk`: umbrella crate that re-exports the workspace crates.
- `scratch`: local binary crate for ad-hoc experiments (not part of SDK surface).

## Documentation

- Docs index: [`docs/README.md`](docs/README.md)
- API reference: [`docs/api-reference.md`](docs/api-reference.md)
- Create guide: [`docs/create-did.md`](docs/create-did.md)
- Dereference guide: [`docs/dereference-did.md`](docs/dereference-did.md)
- Testing guide: [`docs/testing.md`](docs/testing.md)
- Architecture notes: [`ARCHITECTURE.md`](ARCHITECTURE.md)

## Prerequisites

- Rust + Cargo (workspace is pinned to `nightly` via [`rust-toolchain.toml`](rust-toolchain.toml))
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
cargo test -p hiero-did-anoncreds --test integration_anoncreds -- --nocapture
cargo test -p hiero-did-sdk --test integration_anoncreds -- --nocapture
```

## Quick Start (Create + Resolve)

```rust
use hiero_did_core::did::Network;
use hiero_did_registrar::create::create_did;
use hiero_did_resolver::{DidDocumentBuilder, MirrorNodeClient};
use hiero_sdk::{AccountId, Client, PrivateKey};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let account_id = AccountId::from_str("0.0.12345")?;
    let operator_key = PrivateKey::from_str_der("<DER_PRIVATE_KEY>")?;

    let client = Client::for_testnet();
    client.set_operator(account_id, operator_key);

    let created = create_did(&client, Network::Testnet, None).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let mirror = MirrorNodeClient::for_testnet();
    let messages = mirror.get_topic_messages(&created.did.topic_id).await?;
    let resolution = DidDocumentBuilder::from(messages).resolve(&created.did).await?;

    println!("Created DID: {}", created.did);
    println!("Resolved DID: {}", resolution.did_document.id);
    Ok(())
}
```

## Using the Umbrella Crate

```rust
use hiero_did_sdk::{anoncreds, client, core, hcs, messages, method, registrar, resolver, signer};
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

## Current Boundaries

- Vault-backed signing is feature-gated and uses blocking HTTP internally to fit the synchronous `Signer` trait.
- Live Vault and live Hedera integration tests require external services and credentials.
- No generic lifecycle-engine crate equivalent to the JS monorepo `lifecycle` package.
