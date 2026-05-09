# Testing Guide

This project has two practical test modes:

- Local/offline checks (no Hedera network calls).
- Hedera integration tests (requires network + credentials).

## Prerequisites

- Rust + Cargo installed.
- For integration tests:
  - outbound network access
  - `.env` file in repo root with:

```env
HEDERA_ACCOUNT_ID=0.0.xxxxx
HEDERA_PRIVATE_KEY=302e020100300506032b657004220420...
```

`HEDERA_PRIVATE_KEY` must be DER format because tests parse it via `PrivateKey::from_str_der`.

## Run All Tests

```bash
cargo test --workspace
```

Behavior:
- Runs unit tests for all workspace crates.
- Also runs `registrar/tests/integration_test.rs`.

## Run Local-Only Tests

Skip integration tests by excluding the registrar crate:

```bash
cargo test --workspace --exclude hiero-did-registrar
```

Useful for CI environments without Hedera access.

## Run Integration Tests Only

```bash
cargo test -p hiero-did-registrar --test integration_test -- --nocapture
```

Current integration tests:

- `test_create_did`: validates end-to-end DID creation.
- `test_create_and_resolve_did`: creates DID then resolves from mirror node.

## Useful Dev Checks

```bash
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

## Common Failures

### Missing credentials

- `HEDERA_ACCOUNT_ID not set`
- `HEDERA_PRIVATE_KEY not set`

Fix: ensure `.env` exists and has both keys.

### Invalid key format

- `Invalid private key`

Fix: provide DER-encoded private key string.

### Network blocked or restricted

- `grpc: Status { code: Unavailable, message: "tcp open error" ... }`

Fix: allow outbound network to Hedera endpoints.

### Mirror-node eventual consistency

- resolve test may fail if messages are not visible yet.

Fix: rerun; mirror-node lag can be transient.
