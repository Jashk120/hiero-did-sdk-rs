# Architecture: `hiero-did-lifecycle`

## Purpose

`hiero-did-lifecycle` is a small Rust orchestration crate for ordered, labeled operation pipelines. It is generic over the message type and context type, so it can support DID-related workflows without knowing how a DID message is built, how HCS publication works, or where key material is stored.

The crate depends only on `hiero-did-core`.

## Package Boundary

Workspace location:

- `lifecycle`

Public source files:

- `src/lib.rs`: public exports.
- `src/builder.rs`: `LifecycleBuilder`.
- `src/message.rs`: `LifecycleMessage`.
- `src/runner.rs`: `LifecycleRunner` and `LifecycleRunnerOptions`.
- `src/state.rs`: `RunnerState` and `RunnerStatus`.
- `src/step.rs`: `LifecycleStep` and `LifecycleStepKind`.
- `src/types.rs`: boxed future and callback type aliases.

Tests:

- `tests/lifecycle.rs`: builder validation, signing, external signature pause/resume, hooks, and catch handling.

## Public API

### `LifecycleMessage`

Messages processed by the runner implement:

```rust
pub trait LifecycleMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError>;
    fn set_signature(&mut self, signature: Vec<u8>) -> Result<(), DIDError>;
}
```

`message_bytes` is used by signer-backed steps. `set_signature` is used by both signer-backed and externally attached signature steps.

### `LifecycleBuilder<M, C = ()>`

`LifecycleBuilder` owns an ordered list of labeled steps for message type `M` and context type `C`.

Step builders:

- `callback(label, callback)`: run custom async logic against `&mut M` and `&mut C`.
- `sign_with_signer(label)`: sign `message.message_bytes()` using a supplied `core::Signer`, then attach the signature to the message.
- `attach_signature(label)`: attach a caller-provided signature from runner options.
- `pause(label)`: return a resumable pause state.
- `catch(label, callback)`: install a global error handler for the pipeline.

Lookup helpers:

- `len`
- `is_empty`
- `get_by_index`
- `get_by_label`
- `get_index_by_label`
- `catch_label`

Normal pipeline labels must be unique. The catch handler is stored separately from normal steps.

### `LifecycleRunner<M, C = ()>`

`LifecycleRunner` executes a completed builder.

Methods:

- `process(message, options)`: starts at the first step.
- `resume(state, options)`: continues after a pause state's label.
- `on_complete(label, hook)`: registers hooks that run after a labeled step completes.

`resume` only accepts `RunnerStatus::Pause` states.

### `LifecycleRunnerOptions<'a, C>`

Runtime inputs supplied to the runner:

- `context: C`
- `signer: Option<&'a dyn Signer>`
- `signature: Option<Vec<u8>>`

`LifecycleRunnerOptions::new(context)` initializes a run with no signer and no attached signature.

### `RunnerState<M>`

Every successful runner invocation returns a state:

```rust
pub struct RunnerState<M> {
    pub message: M,
    pub status: RunnerStatus,
    pub index: isize,
    pub label: String,
    pub error: Option<String>,
}
```

State meanings:

- `Success`: pipeline reached the end. `index` is `-1`, `label` is empty.
- `Pause`: pipeline stopped at a pause step. `index` and `label` identify the pause.
- `Error`: pipeline error was handled by a catch step. `index` is `-1`, `label` is empty, and `error` contains the original error text.

If a step fails and no catch step exists, the runner returns the original `DIDError`.

## Runtime Model

The runner processes a mutable message through a linear step list.

```text
LifecycleBuilder --> ordered LifecycleStep list --> LifecycleRunner
LifecycleMessage ------------------------------------^
LifecycleRunnerOptions(context/signer/signature) -----^
```

Execution rules:

1. Determine the start index. Fresh runs start at `0`; resumed runs start after the pause label.
2. Execute each step in order.
3. Run completion hooks after completed callback, signer, and attach-signature steps.
4. For resumed runs, run hooks for the pause label before executing the next step.
5. Return `RunnerStatus::Pause` immediately when a pause step is reached.
6. Return `RunnerStatus::Success` after the final step.
7. If a step returns `DIDError`, call the catch handler when configured; otherwise return the error.

The message is moved into `process`, mutated in place during execution, and returned inside `RunnerState`.

## Step Types

### Callback

Callback steps run user-supplied async logic:

```rust
Fn(&mut M, &mut C) -> LifecycleFuture<Result<(), DIDError>>
```

These are used for operation-specific work such as building state, checking invariants, publishing payloads, or updating context.

### Sign With Signer

Signer steps require `options.signer`.

The runner:

1. Calls `message.message_bytes()`.
2. Calls `signer.sign_bytes(&bytes)`.
3. Calls `message.set_signature(signature)`.

Missing signer input returns `DIDError::InvalidArgument`.

### Attach Signature

Attach-signature steps require `options.signature`.

The runner consumes the signature with `take()` and calls `message.set_signature(signature)`. Missing signature input returns `DIDError::InvalidArgument`.

### Pause

Pause steps return:

```rust
RunnerState {
    status: RunnerStatus::Pause,
    index,
    label,
    ...
}
```

Pause hooks run only when that pause state is resumed, immediately before the next step executes.

### Catch

Catch handlers are global to one builder. When configured, they convert step failures into `RunnerStatus::Error` states after the handler succeeds.

The catch callback receives the original `DIDError`. It does not receive the message or context.

## Typical External Signature Flow

```text
process(message)
  -> pause("pause-for-signature")

external signer signs message bytes

resume(paused_state, options.signature = Some(signature))
  -> attach_signature("signature")
  -> pause("pause-before-publish")

resume(paused_state)
  -> callback("publish")
  -> success
```

This shape lets callers separate message construction, external signing, optional pre-publish coordination, and publication.

## Current Boundaries

- The crate is domain-neutral. DID/HCS semantics live in neighboring crates.
- The crate does not verify signatures. Verification belongs to message-specific code or higher-level workflows.
- The crate does not publish transactions or messages.
- Completion hooks run sequentially in registration order.
