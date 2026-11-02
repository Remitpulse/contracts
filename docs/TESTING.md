# Testing

Run `make ci` for formatting, Clippy, workspace tests, deployment preflight
tests, release WASM build, and account WASM tests. `make test` runs native Rust
and stubbed deployment tests. No test submits a transaction.

Account coverage includes passkey quorum, session scope, limits, expiry, pause,
rollback, and authorization nonce replay. Subsidy coverage includes admin access,
duplicate IDs, insufficient funds, and retry. Router coverage includes caller
authorization and atomicity.
