# Docs

Focused guides for working with this SDK workspace.

## Workspace Baseline

- Rust workspace pinned to `stable` via `rust-toolchain.toml`.
- Integration tests use `.env.local` in repo root (preferred), with `.env` as fallback.

## Guides

- [`api-reference.md`](./api-reference.md): public API summary by crate.
- [`sdk.md`](./sdk.md): top-level `HieroDidSdk` orchestrator guide with unified code examples.
- [`create-did.md`](./create-did.md): local-key and external-signer create flows plus related write-operation context.
- [`resolve-did.md`](./resolve-did.md): DID resolution with TopicReader abstraction, transport options, polling helpers, and representation negotiation.
- [`dereference-did.md`](./dereference-did.md): DID URL parse + dereference flow.
- [`csm.md`](./csm.md): client-side message signing prepare/sign/submit flow.
- [`testing.md`](./testing.md): local checks, feature-gated Vault signer checks, and networked integration test setup.

## Recommended Read Order

1. Start with `sdk.md` to understand the main developer entrypoint (`HieroDidSdk`).
2. Read `create-did.md` for the write-path mental model.
3. Read `resolve-did.md` for the resolution pipeline and transport options.
4. Read `dereference-did.md` for fragment/resource dereference behavior.
5. Use `api-reference.md` while integrating crate APIs.
6. Read `testing.md` before running integration suites.
