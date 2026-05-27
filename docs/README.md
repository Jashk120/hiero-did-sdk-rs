# Docs

Focused guides for working with this SDK workspace.

## Workspace Baseline

- Rust workspace pinned to `nightly` via `rust-toolchain.toml`.
- Integration tests use `.env.local` in repo root (preferred), with `.env` as fallback.
- Local `scratch` crate exists for experiments and is not part of the SDK public surface.

## Guides

- [`api-reference.md`](./api-reference.md): public API summary by crate.
- [`create-did.md`](./create-did.md): local-key and external-signer create flows plus related write-operation context.
- [`dereference-did.md`](./dereference-did.md): DID URL parse + dereference flow.
- [`testing.md`](./testing.md): local checks, feature-gated Vault signer checks, and networked integration test setup.

## Recommended Read Order

1. Start with `create-did.md` for the write-path mental model.
2. Read `dereference-did.md` for fragment/resource dereference behavior.
3. Use `api-reference.md` while integrating crate APIs.
4. Read `testing.md` before running integration suites.
