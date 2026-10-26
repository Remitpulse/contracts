# RemitPulse router

See the [workspace README](../../README.md) for build, test, optimization and deployment instructions.

## Contract interface reference

For a maintained reference of every public entrypoint, authorization requirement, input validation expectation, storage effect, and emitted event, see the [contract interface and event reference](../../docs/CONTRACT_INTERFACE.md).

## Batch limits

`Router.exec` accepts a batch of contract invocations. To bound CPU, memory, and event/result resource consumption, the following limits are enforced before any invocation is dispatched:

- **Maximum invocations per `exec` call:** `MAX_INVOCATIONS` (see `contracts/router/src/lib.rs` for the current value). A batch larger than this is rejected.
- **Maximum argument size per invocation:** `MAX_ARG_SIZE` bytes (see `contracts/router/src/lib.rs` for the current value). Any single invocation whose serialized arguments exceed this size is rejected.

When a limit is exceeded, `exec` returns a typed error before dispatching any invocation:

- `RouterError::TooManyInvocations` — the batch contains more than `MAX_INVOCATIONS` invocations.
- `RouterError::ArgumentTooLarge` — at least one invocation's arguments exceed `MAX_ARG_SIZE` bytes.

Accepted batches execute **atomically**: either every invocation in the batch succeeds, or the entire call reverts and no state changes are persisted.