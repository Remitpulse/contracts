# Architecture

RemitPulse is a Rust workspace of independent Soroban contracts. `core` is only
a deployment convenience group, not an on-chain dependency.

- `account`: WebAuthn quorum custom account with bounded Ed25519 sessions.
- `webauthn`: standalone shared ES256 verifier.
- `subsidy`: administrator-controlled SEP-41 reimbursement vault.
- `router`: atomic, caller-authorized generic invocation batch.
- `airdrop` and `nft`: extracted OpenZeppelin-derived components.

Soroban atomicity rolls back prior writes and calls when an invocation fails.
