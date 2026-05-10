# Testing Guide

This workspace has both local checks and networked integration tests.

## Prerequisites

- Rust + Cargo installed.
- For integration tests, set `.env` in repo root:

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
HEDERA_NETWORK=testnet
```

Notes:

- Registrar integration tests parse `HEDERA_PRIVATE_KEY` with `PrivateKey::from_str_der`.
- HCS/client integrations parse keys using `PrivateKey::from_str`.

## Run All Tests

```bash
cargo test --workspace
```

## Run Local Checks

```bash
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

## Run Integration Tests by Crate

```bash
cargo test -p hiero-did-client --test client_service_integration -- --nocapture
cargo test -p hiero-did-hcs --test integration_hcs -- --nocapture
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
```

## Integration Coverage (Current)

- `hiero-did-client`
  - service initialization and network selection behavior.
- `hiero-did-hcs`
  - topic create/update/delete, message publish/read, file publish/resolve, cache + service paths.
- `hiero-did-registrar`
  - create DID and create+resolve flow.

## Common Failures

- `HEDERA_ACCOUNT_ID not set` / `HEDERA_PRIVATE_KEY not set`
  - `.env` missing or incomplete.
- `Invalid private key`
  - Key format does not match parser used by that test.
- `grpc: Status { code: Unavailable, ... }`
  - Outbound network blocked or unstable.
- `DID document not found` during resolve flow
  - Mirror node lag; retry after a short wait.
