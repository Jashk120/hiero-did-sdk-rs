# Testing Guide

This workspace has both local checks and networked integration tests.

## Prerequisites

- Rust + Cargo installed (workspace is pinned to `nightly` via `rust-toolchain.toml`).
- For integration tests, set `.env.local` in repo root (preferred). `.env` is also supported as fallback.

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
HEDERA_NETWORK=testnet
```

Notes:

- Different integration tests may parse private keys with different constructors.
- Keep key format consistent with the specific test suite.

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
cargo test -p hiero-did-anoncreds --test integration_anoncreds -- --nocapture
cargo test -p hiero-did-sdk --test integration_anoncreds -- --nocapture
```

## Integration Coverage (Current)

- `hiero-did-client`
- service initialization and network selection behavior.
- `hiero-did-hcs`
- topic create/update/delete, message publish/read, file publish/resolve, cache + service paths.
- `hiero-did-registrar`
- DID write flows around create, update (add/remove verification methods and services), and deactivate.
- `hiero-did-anoncreds`
- schema/cred-def/revocation registry operations on HCS.
- `hiero-did-sdk`
- re-export wiring and SDK-level anoncreds integration coverage.

## Common Failures

- `HEDERA_ACCOUNT_ID not set` / `HEDERA_PRIVATE_KEY not set`
- `.env` missing or incomplete.
- `Invalid private key`
- Key format does not match parser used by that test.
- `grpc: Status { code: Unavailable, ... }`
- Outbound network blocked or unstable.
- `DID document not found` during resolve flow
- Mirror node lag; retry after a short wait.
