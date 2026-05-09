# hiero-did-sdk-rs

Rust workspace for creating and resolving `did:hedera` identifiers over Hedera Consensus Service (HCS).

## Documentation

- Docs index: [`docs/README.md`](docs/README.md)
- Create DID guide: [`docs/create-did.md`](docs/create-did.md)
- API reference: [`docs/api-reference.md`](docs/api-reference.md)
- Testing guide: [`docs/testing.md`](docs/testing.md)

## Workspace Crates

- `hiero-did-core`: DID types, DID document models, errors, and key utilities.
- `hiero-did-method`: DID parsing and validation helpers.
- `hiero-did-messages`: HCS message, envelope, and event models.
- `hiero-did-signer`: internal Ed25519 sign/verify helpers.
- `hiero-did-hcs`: Hedera client/topic helpers for HCS operations.
- `hiero-did-registrar`: high-level DID creation workflow.
- `hiero-did-resolver`: mirror-node fetch + DID document reconstruction.
- `hiero-did-sdk`: umbrella crate that re-exports all crates above.

## Prerequisites

- Rust stable toolchain (workspace uses edition `2024`)
- Cargo
- Outbound network access for integration tests
- Hedera test account credentials for integration tests

Optional `.env` for integration tests:

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
```

`HEDERA_PRIVATE_KEY` must be DER format (`PrivateKey::from_str_der`).

## Build

```bash
cargo build --workspace
```

## Test

Run all tests:

```bash
cargo test --workspace
```

Run integration tests only:

```bash
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
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

    // Mirror node is eventually consistent; wait briefly before resolving.
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

## Using The Umbrella SDK Crate

If you depend on `hiero-did-sdk`, use re-exported modules:

```rust
use hiero_did_sdk::{core, registrar, resolver};
```

## Troubleshooting

- `HEDERA_ACCOUNT_ID not set` / `HEDERA_PRIVATE_KEY not set`
  - Ensure `.env` exists and both values are present.
- `Invalid private key`
  - Use DER-encoded private key for `PrivateKey::from_str_der`.
- `grpc: Status { code: Unavailable, ... }`
  - Usually outbound network restrictions.
- `DID document not found`
  - Mirror node may still be catching up after creation; retry after a short wait.
