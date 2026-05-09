# Testing Guide

This project has two test modes:

- Local/offline checks.
- Hedera integration tests (network + credentials required).

## Prerequisites

- Rust + Cargo installed.
- For integration tests, set `.env` in repo root:

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
```

`HEDERA_PRIVATE_KEY` must be DER format (`PrivateKey::from_str_der`).

## Run All Tests

```bash
cargo test --workspace
```

This includes `registrar/tests/integration_test.rs`.

## Run Local-Only Checks

```bash
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

## Run Integration Tests Only

```bash
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
```

Current integration coverage:

- `test_create_did`
- `test_create_and_resolve_did`

## Common Failures

- `HEDERA_ACCOUNT_ID not set` / `HEDERA_PRIVATE_KEY not set`
  - `.env` missing or incomplete.
- `Invalid private key`
  - Key is not DER-encoded.
- `grpc: Status { code: Unavailable, ... }`
  - Outbound network blocked or unstable.
- `DID document not found` during resolve flow
  - Mirror node lag; wait and retry.
