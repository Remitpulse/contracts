# RemitPulse airdrop

See the [workspace README](../../README.md) for build, test, optimization and deployment instructions.

## Contract interface reference

For the maintained reference of every public entrypoint, authorization and input
validation expectations, storage effects, emitted events, and errors, see the
[contract interface reference](../../docs/contract-interface-reference.md).

When changing any public entrypoint, event, or error in this contract, update the
interface reference in the same pull request. Documentation review is part of the
pull-request expectations.

## Claim deadline and recovery policy

The airdrop contract enforces an explicit, recipient-visible claim window so that an
administrator cannot recover unclaimed funds while recipients are still meant to be
claiming.

### Constructor `claim_deadline` parameter

The constructor takes a `claim_deadline` parameter (a ledger timestamp). This value is
immutable for the lifetime of the instance and defines the end of the claim window:

- Recipients may claim until the ledger timestamp reaches `claim_deadline`.
- The deadline is published on-chain at construction so recipients can verify the
  schedule before participating.

### Recovery is rejected before the deadline

`recover_unclaimed` may only be called after the claim deadline has passed. Any attempt
to recover before the deadline is rejected with a typed error (see the
[contract interface reference](../../docs/contract-interface-reference.md) for the
exact error variant). This guarantees the administrator cannot sweep funds during the
claim window.

### `recovered` event

When recovery succeeds, the contract emits a `recovered` event. This event finalizes
the instance and makes the recovery visible to recipients and indexers. The event
payload is documented in the
[contract interface reference](../../docs/contract-interface-reference.md).

### Instance TTL lifecycle

The contract maintains an explicit instance TTL policy rather than relying on default
storage lifetime:

- The instance TTL is extended at construction.
- The instance TTL is renewed on every state-changing call, including claims and
  `recover_unclaimed`.

The configured threshold and extension behavior are covered by tests. See the
[contract interface reference](../../docs/contract-interface-reference.md) for the
exact TTL constants and their semantics.

### Deployment validation

Deployment tooling validates the `claim_deadline` argument (for example, that it is a
valid future timestamp relative to deployment) before submitting the constructor
invocation. See the workspace deployment instructions in the
[workspace README](../../README.md) for details.