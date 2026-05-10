# hiero-did-sdk-rs

Rust workspace for creating and resolving `did:hedera` identifiers, plus reusable Hedera client and HCS service layers.

## Workspace Crates

- `hiero-did-core`: canonical DID types, document models, errors, and key utilities.
- `hiero-did-method`: parser/validator helpers for `did:hedera` and topic IDs.
- `hiero-did-messages`: signed envelope + DID owner event payload models.
- `hiero-did-signer`: internal Ed25519 sign/verify helpers.
- `hiero-did-client`: configurable Hedera client service for single or multi-network setups.
- `hiero-did-hcs`: topic/message/file helpers and higher-level HCS service with optional cache.
- `hiero-did-registrar`: end-to-end DID creation flow.
- `hiero-did-resolver`: mirror-node reader + DID document reconstruction.
- `hiero-did-sdk`: umbrella crate that re-exports core DID crates and higher-level registrar/resolver/hcs modules.

## Documentation

- Docs index: [`docs/README.md`](docs/README.md)
- Create DID guide: [`docs/create-did.md`](docs/create-did.md)
- API reference: [`docs/api-reference.md`](docs/api-reference.md)
- Testing guide: [`docs/testing.md`](docs/testing.md)
- Architecture notes: [`ARCHITECTURE.md`](ARCHITECTURE.md)

## Prerequisites

- Rust stable toolchain (workspace uses edition `2024`)
- Cargo
- Outbound network access for integration tests
- Hedera test account credentials for integration tests

Optional `.env` in repo root:

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
HEDERA_NETWORK=testnet
```

Notes:

- `HEDERA_PRIVATE_KEY` is parsed by `hiero_sdk::PrivateKey::from_str` in `hiero-did-client` and by `PrivateKey::from_str_der` in registrar integration tests.
- Keep the key format consistent with the tests you run.

## Build

```bash
cargo build --workspace
```

## Test

Run all tests:

```bash
cargo test --workspace
```

Run registrar integration tests only:

```bash
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
```

Run HCS integration tests only:

```bash
cargo test -p hiero-did-hcs --test integration_hcs -- --nocapture
```

Run local checks:

```bash
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
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

    // Mirror node is eventually consistent.
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let mirror = MirrorNodeClient::for_testnet();
    let messages = mirror.get_topic_messages(&created.did.topic_id).await?;
    let resolution = DidDocumentBuilder::from(messages)
        .resolve(&created.did)
        .await?;

    println!("Created DID: {}", created.did);
    println!("Resolved DID: {}", resolution.did_document.id);
    Ok(())
}
```

## Using the Umbrella Crate

```rust
use hiero_did_sdk::{core, hcs, registrar, resolver};
```

## Troubleshooting

- `HEDERA_ACCOUNT_ID not set` / `HEDERA_PRIVATE_KEY not set`
  - Ensure `.env` exists and values are present.
- `Invalid private key`
  - Check whether your path expects DER (`from_str_der`) or standard SDK parse format (`from_str`).
- `grpc: Status { code: Unavailable, ... }`
  - Usually outbound network restrictions or unstable connectivity.
- `DID document not found`
  - Mirror node may still be catching up after creation; retry after a short delay.
