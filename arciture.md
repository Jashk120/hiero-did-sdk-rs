# Legacy Reference: Hiero DID SDK JS Packages Architecture (`packages/`)

> Note: This file documents the older JavaScript/TypeScript monorepo layout and does **not** describe the Rust workspace in this repository.  
> For current Rust workspace architecture, use `ARCHITECTURE.md` and `README.md`.

This document explains what each package and file set does, how data flows across packages, and what is intentionally out of scope.

## 1. Monorepo Structure

The `packages/` folder is a multi-package TypeScript monorepo.

Common file patterns across packages:
- `src/**`: implementation code.
- `tests/**`: unit/integration/e2e tests.
- `README.md`: package-level usage docs.
- `package.json`: package metadata, exports, deps.
- `tsconfig*.json`: TS build/test config.
- `tsdown.config.ts`: bundling config.
- `vitest.config.ts`: test runner config.
- `CHANGELOG.md`: release history.
- `LICENSE`: license text (present in several packages).

Generated/build folders not documented line-by-line here:
- `dist/**` (compiled outputs)
- `node_modules/**` (dependencies)

## 2. High-Level Runtime Architecture

Core flow:
1. `registrar` creates/updates/deactivates DIDs by composing `messages`, signing them, and publishing to Hedera.
2. `hcs` handles topic/message/file operations against Hedera Consensus Service.
3. `resolver` reads DID topic events and reconstructs DID Documents.
4. `core` provides shared contracts/types/utilities used by all packages.
5. `signer-*` and `verifier-*` provide cryptographic backends (internal key material or Vault).
6. `publisher-internal` submits Hedera transactions.
7. `anoncreds` builds an AnonCreds registry layer on top of HCS.
8. `client`, `cache`, `crypto`, `zstd`, `lifecycle` are supporting utility packages.

## 3. Package-by-Package File Map

## `core/`
Purpose: foundational interfaces, parsers, validators, codecs, constants.

Important files:
- `src/index.ts`: main export barrel.
- `src/constants.ts`: constants like DID root key id.
- `src/interfaces/*.ts`: shared contracts (`Signer`, `Verifier`, `Publisher`, DID types, error model, network/cache types).
- `src/parsers/parse-did.ts`: DID parser.
- `src/parsers/parse-did-url.ts`: DID URL parser.
- `src/parsers/index.ts`: parser exports.
- `src/validators/is-did.ts`: DID and DID URL validation helpers.
- `src/validators/is-ed25519-public-key.ts`: Ed25519 pubkey shape validation.
- `src/validators/index.ts`: validator exports.
- `src/utils/cbor-codec.ts`: CBOR encode/decode helper.
- `src/utils/varint-codec.ts`: varint codec wrapper.
- `src/utils/multibase-codec.ts`: multibase encode/decode helper.
- `src/utils/keys-utility.ts`: key format conversion/utility helpers.
- `src/utils/index.ts`: utility exports.

Tests/config/docs:
- `tests/*.spec.ts`: coverage of parsers/validators/codecs/contracts.
- `tests/helpers.ts`: shared test helpers.
- `tests/tsconfig.json`, `vitest.config.ts`, `tsconfig*.json`, `tsdown.config.ts`.
- `README.md`, `CHANGELOG.md`, `LICENSE`, `package.json`.

Does not do:
- No network I/O.
- No Hedera transaction submission.

## `client/`
Purpose: Hedera SDK client construction and network configuration.

Important files:
- `src/hedera-client.configuration.ts`: network config types/defaults.
- `src/hedera-client-service.ts`: `HederaClientService` lifecycle/lookup of clients by network.
- `src/index.ts`: exports.

Tests/config/docs:
- `tests/hedera-client-service.e2e.ts`.
- `tests/tsconfig.json`, `vitest.config.ts`, `tsconfig*.json`, `tsdown.config.ts`.
- `README.md`, `CHANGELOG.md`, `package.json`.

Does not do:
- No DID message formatting.
- No DID resolution logic.

## `hcs/`
Purpose: Hedera Consensus Service abstraction (topics, messages, HCS-1 files) + caching + mirror helpers.

Important files:
- `src/index.ts`: top-level exports.
- `src/hedera-hcs-service.configuration.ts`: config type.
- `src/hedera-hcs-service.ts`: facade service wiring lower-level services.
- `src/hcs/hcs-topic-service.ts`: create/update/delete/get topic info.
- `src/hcs/hcs-message-service.ts`: submit/read topic messages.
- `src/hcs/hcs-file-service.ts`: chunked file submit/resolve over HCS-1 style messaging.
- `src/hcs/index.ts`: HCS service exports.
- `src/cache/hcs-cache-service.ts`: topic/message cache layer.
- `src/cache/index.ts`: cache exports.
- `src/shared/changes-awaiter.ts`: visibility polling/retry helper.
- `src/shared/mirror-node.ts`: mirror-node capability detection helper.
- `src/shared/index.ts`: shared exports.

Tests/config/docs:
- `tests/unit/*.spec.ts`: topic/message/file/cache/helper coverage.
- `tests/integration/hedera-hcs-service.e2e.ts`.
- configs/docs metadata files as above.

Does not do:
- No DID document reconstruction semantics.
- No key custody/signature generation.

## `messages/`
Purpose: typed DID message models + operation lifecycles for create/update/deactivate sub-operations.

Top-level files:
- `src/index.ts`: package export root.
- `src/messages/index.ts`: exports all message families.
- `src/validators/id-property-id.ts`, `is-topic-id.ts`, `is-uri.ts`: input validation helpers.

Per-message families (same structure):
- `did-owner/`
- `did-deactivate/`
- `did-add-verification-method/`
- `did-remove-verification-method/`
- `did-add-service/`
- `did-remove-service/`

Each family contains:
- `message.ts`: concrete `DIDMessage` subclass serialization/deserialization rules.
- `interfaces.ts`: constructor and serialized payload types.
- `lifecycle/default.ts`: default (direct) lifecycle flow.
- `lifecycle/client-mode.ts`: client-side message signing flow.
- `lifecycle/index.ts`: lifecycle exports.
- `index.ts`: family export barrel.

Special extra file:
- `did-owner/lifecycle/context.ts`: extra context model used by owner message lifecycle.
- `did-owner/utils.ts`: helper to check DID existence.

Tests/config/docs:
- `tests/unit/**`: each message family + lifecycle mode tests.
- validator tests and shared helper.

Does not do:
- No direct transaction publishing.
- No DID topic reading from network (delegates to resolver/topic readers where needed).

## `resolver/`
Purpose: resolve and dereference Hedera DID data into DID documents/resolution outputs.

Important files:
- `src/index.ts`: exports resolve/dereference APIs and supporting types.
- `src/get-resolver.ts`: resolver adapter shape for resolver ecosystems.
- `src/resolve-did.ts`: primary DID resolution function.
- `src/dereference-did.ts`: DID URL dereference entrypoint.
- `src/did-document-builder.ts`: reconstructs DID document from event stream.
- `src/did-dereference-builder.ts`: resolves DID URL fragments/resources.
- `src/topic-readers/topic-reader-hedera-client.ts`: reader via Hedera SDK client.
- `src/topic-readers/topic-reader-hedera-hcs.ts`: reader via HCS service abstraction.
- `src/topic-readers/topic-reader-hedera-rest-api.ts`: reader via mirror REST API.
- `src/topic-readers/index.ts`: topic reader exports.
- `src/helpers/parse-did.ts`, `parse-did-url.ts`, `index.ts`: parsing helpers.
- `src/interfaces/*.ts`: event payload, reader, accept, resolve options types.
- `src/validators/*.ts`: runtime guards for event/message forms.

Tests/config/docs:
- tests for resolve/dereference/builders/helpers/readers/validators.

Does not do:
- No DID write operations.
- No private-key signing.

## `registrar/`
Purpose: DID write operations (`createDID`, `updateDID`, `deactivateDID`) with both immediate and CSM (client-side message) modes.

Top-level:
- `src/index.ts`: exports operation modules.
- `src/interfaces/*.ts`: shared registrar options/provider/state/request models.
- `src/shared/get-*.ts`: resolve signer/publisher/provider options/root-key dependencies.
- `src/shared/message-awaiter.ts`: wait helpers for message visibility.
- `src/shared/index.ts`: shared exports.
- `src/utils/did-update-builder.ts`: update request composition helper.

Create flow:
- `src/create-did/interface.ts`: create operation contracts.
- `src/create-did/operation.ts`: create operation implementation.
- `src/create-did/csm-operation.ts`: client-side message create request generation/submission.
- `src/create-did/utils.ts`: create helper logic.
- `src/create-did/index.ts`: exports.

Update flow:
- `src/update-did/interface.ts`: update contracts.
- `src/update-did/operation.ts`: update execution flow.
- `src/update-did/csm-operation.ts`: client-side message update flow.
- `src/update-did/helpers/deserialize-state.ts`: recover operation state.
- `src/update-did/helpers/fragment-search.ts`: DID fragment lookup helper.
- `src/update-did/helpers/have-id.ts`: ID checks.
- `src/update-did/sub-operations/*.ts`: per-update mutation handlers:
  - `add-service.ts`
  - `remove-service.ts`
  - `add-verification-method.ts`
  - `remove-verification-method.ts`
  - `index.ts` (dispatcher)
  - `interfaces.ts` (sub-operation handler contracts)
- `src/update-did/index.ts`: exports.

Deactivate flow:
- `src/deactivate-did/interface.ts`
- `src/deactivate-did/operation.ts`
- `src/deactivate-did/csm-operation.ts`
- `src/deactivate-did/index.ts`

Tests/config/docs:
- tests grouped by `create-did/`, `update-did/`, `deactivate-did/`, and `shared/`.

Does not do:
- No DID resolution reconstruction logic.
- No storage of long-term operation state outside returned CSM state payloads.

## `lifecycle/`
Purpose: generic workflow engine for DID message execution orchestration.

Files:
- `src/builder.ts`: fluent lifecycle definition builder.
- `src/runner.ts`: executes lifecycle steps (callback/signature/pause/catch).
- `src/interfaces/steps.ts`: step types.
- `src/interfaces/runner-state.ts`: runner state model.
- `src/interfaces/hooks.ts`: hook function type.
- `src/interfaces/index.ts`: interface exports.
- `src/index.ts`: package exports.

Tests/config/docs:
- builder + runner tests and helper.

Does not do:
- No DID domain logic itself; only execution framework.

## `cache/`
Purpose: in-memory LRU cache implementation.

Files:
- `src/LRUCache.ts`: `LRUMemoryCache` implementation.
- `src/index.ts`: exports.

Tests/config/docs:
- cache behavior tests and standard config/docs files.

Does not do:
- No persistence.
- No distributed cache semantics.

## `crypto/`
Purpose: cross-runtime SHA-256 helper.

Files:
- `src/crypto.ts`: `Crypto` facade and module selection.
- `src/node-crypto.ts`: Node runtime crypto adapter.
- `src/react-native-crypto.ts`: RN/runtime adapter lookup.
- `src/index.ts`: exports.

Tests/config/docs:
- unit + e2e tests.

Does not do:
- No asymmetric signing/verifying interfaces.

## `zstd/`
Purpose: cross-runtime zstd compress/decompress helper.

Files:
- `src/zstd.ts`: `Zstd` facade and module selection.
- `src/node-zstd.ts`: Node adapter.
- `src/react-native-zstd.ts`: RN adapter.
- `src/index.ts`: exports.

Tests/config/docs:
- unit + e2e tests.

Does not do:
- No alternate compression algorithms.

## `signer-internal/`
Purpose: Ed25519 signing implementation backed by local Hedera SDK keys.

Files:
- `src/signer.ts`: concrete signer implementation.
- `src/validators.ts`: private-key format/type guards.
- `src/index.ts`: exports.

Tests/config/docs:
- signer tests and standard package metadata.

Does not do:
- No external key-management system integration.

## `verifier-internal/`
Purpose: Ed25519 verification implementation backed by local/public key material.

Files:
- `src/verifier.ts`: verifier implementation.
- `src/index.ts`: exports.

Tests/config/docs:
- verifier tests and standard metadata.

Does not do:
- No key custody or key generation.

## `signer-hashicorp-vault/`
Purpose: Signer implementation using Vault transit keys and auth flows.

Files:
- `src/vault-api.ts`: HTTP client for Vault auth/key/sign/verify operations.
- `src/vault-signer-factory.ts`: factory to authenticate and create signer instances.
- `src/signer.ts`: signer implementation using Vault API.
- `src/interfaces.ts`: Vault login and signer options.
- `src/utils.ts`: URL/path normalization helpers.
- `src/index.ts`: exports.

Tests/config/docs:
- tests for API, utils, factory, signer.

Does not do:
- No local private key signing fallback in this package.

## `verifier-hashicorp-vault/`
Purpose: Verifier implementation using Vault key operations.

Files:
- `src/vault-api.ts`: Vault HTTP operations.
- `src/vault-verifier-factory.ts`: factory to authenticate and create verifier.
- `src/verifier.ts`: verifier implementation.
- `src/interfaces.ts`: verifier/Vault options.
- `src/utils.ts`: URL/path normalization helpers.
- `src/index.ts`: exports.

Tests/config/docs:
- tests for API, utils, factory, verifier.

Does not do:
- No local/offline verification mode in this package.

## `publisher-internal/`
Purpose: concrete Hedera transaction publisher implementation.

Files:
- `src/publisher.ts`: submit/execute transactions against Hedera client.
- `src/index.ts`: exports.

Tests/config/docs:
- publisher tests and metadata.

Does not do:
- No DID message composition.

## `anoncreds/`
Purpose: Hedera AnonCreds registry implementation using HCS.

Files:
- `src/hedera-anoncreds-registry.ts`: main registry API (register/resolve flows).
- `src/hedera-anoncreds-registry.configuration.ts`: configuration type aliasing HCS config.
- `src/index.ts`: package exports.
- `src/specification/*.ts`: AnonCreds object spec interfaces.
- `src/specification/index.ts`: spec export barrel.
- `src/dto/base.ts`: shared DTO utility/state primitives.
- `src/dto/errors.ts`: AnonCreds error type.
- `src/dto/schema.ts`: schema DTOs.
- `src/dto/credential-definition.ts`: cred-def DTOs.
- `src/dto/revocation-registry-definition.ts`: revocation registry definition DTOs.
- `src/dto/revocation-status-list.ts`: revocation status list DTOs.
- `src/dto/revocation-registry-entry.ts`: revocation registry entry DTOs.
- `src/dto/index.ts`: DTO exports.
- `src/utils/identifiers.ts`: AnonCreds identifier parser/builder.
- `src/utils/index.ts`: utils exports.

Tests/config/docs:
- unit tests for registry and helpers.
- integration e2e for registry behavior.

Does not do:
- No generic DID registrar/resolver responsibilities beyond AnonCreds object lifecycle.

## 4. Cross-Package Dependency Intent

- `core` is foundational and has no DID-network orchestration responsibilities.
- `client` and `hcs` provide network access primitives.
- `messages` and `lifecycle` model operation payloads + execution choreography.
- `registrar` is the DID write orchestrator.
- `resolver` is the DID read/reconstruction layer.
- `publisher-internal`, `signer-*`, `verifier-*` are backend implementations of abstract contracts from `core`.
- `anoncreds` is a domain-specific extension package.

## 5. What This Workspace Does Not Cover

- No on-chain smart contract execution model (this is HCS/event-driven DID method tooling).
- No database-backed persistence layer in these packages.
- No REST server/API gateway implementation in `packages/`.
- No key escrow policy engine (Vault packages delegate to Vault policy/config).

## 6. Practical Reading Order

For new contributors, recommended order:
1. `core` (interfaces/parsers/validators)
2. `messages` + `lifecycle`
3. `hcs` + `client`
4. `registrar`
5. `resolver`
6. backend adapters (`signer-*`, `verifier-*`, `publisher-internal`)
7. `anoncreds`
