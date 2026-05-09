# hiero-did-sdk-rs

Rust workspace for creating and resolving `did:hedera` identifiers using Hedera Consensus Service (HCS).

## Documentation

- See [`docs/README.md`](docs/README.md) for topic-specific guides.
- API details: [`docs/api-reference.md`](docs/api-reference.md)
- Creation-only guide: [`docs/create-did.md`](docs/create-did.md)
- Testing guide: [`docs/testing.md`](docs/testing.md)

## What This Repo Contains

This is a Cargo workspace with multiple crates:

- `hiero-did-core`: DID types, document models, key utilities, shared errors.
- `hiero-did-method`: DID parsing and validation helpers.
- `hiero-did-messages`: HCS message/envelope models for DID owner events.
- `hiero-did-hcs`: HCS client/topic helpers.
- `hiero-did-signer`: internal Ed25519 signing and verification.
- `hiero-did-registrar`: DID creation flow (create topic + publish owner message).
- `hiero-did-resolver`: mirror-node fetch + DID document reconstruction.

## Prerequisites

- Rust toolchain (edition `2024` is used; latest stable Rust is recommended)
- Cargo
- Network access for Hedera testnet/mainnet integration tests
- Hedera credentials for integration tests

## Setup

1. Clone the repository.
2. (Optional) Create/update `.env` in repo root for integration tests:

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
```

Notes:
- `HEDERA_PRIVATE_KEY` must be DER format because tests use `PrivateKey::from_str_der`.
- Keep `.env` secret and do not commit real credentials.

## Build

Build all crates:

```bash
cargo build --workspace
```

Build a specific crate:

```bash
cargo build -p hiero-did-registrar
```

## Run Tests

### 1. Run all workspace tests

```bash
cargo test --workspace
```

Important:
- This includes `registrar/tests/integration_test.rs`.
- Integration tests contact Hedera and mirror-node endpoints and require valid `.env` + outbound network access.

### 2. Run only local/unit tests (no Hedera network)

This workspace currently has mostly compile-level coverage and no unit test bodies yet. To skip the integration test binary:

```bash
cargo test --workspace --exclude hiero-did-registrar
```

### 3. Run integration tests only

```bash
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
```

Current integration tests:
- `test_create_did`
- `test_create_and_resolve_did`

## Common Commands

Format:

```bash
cargo fmt --all
```

Lint:

```bash
cargo clippy --workspace --all-targets --all-features
```

Check without building artifacts:

```bash
cargo check --workspace
```

## Minimal Flow (Programmatic)

Typical DID lifecycle in this SDK:

1. Create DID with `hiero_did_registrar::create::create_did`.
2. Wait briefly for mirror-node consistency.
3. Fetch topic messages via `hiero_did_resolver::MirrorNodeClient`.
4. Build DID document via `hiero_did_resolver::DidDocumentBuilder`.

The integration test `registrar/tests/integration_test.rs` is the best end-to-end reference for this flow.

## Project Layout

```text
.
├── Cargo.toml
├── core/
├── method/
├── messages/
├── hcs/
├── signer/
├── registrar/
│   └── tests/integration_test.rs
└── resolver/
```

## Troubleshooting

- `HEDERA_ACCOUNT_ID not set` / `HEDERA_PRIVATE_KEY not set`
  - Ensure `.env` exists and contains both variables.

- `Invalid private key`
  - Verify key is DER-encoded and matches `PrivateKey::from_str_der` expectations.

- `grpc: Status { code: Unavailable, message: "tcp open error" ... }`
  - Usually outbound network restrictions or blocked testnet connectivity.

- `No messages found on topic`
  - Mirror node can lag briefly; rerun after a short wait.
