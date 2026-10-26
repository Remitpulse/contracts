# remitpulse-contracts

Rust/Soroban contracts for RemitPulse, a passkey-powered Stellar remittance app.
This repository contains a Cargo workspace; each contract has its own `Cargo.toml`
and `src/`. SDK 22.0.8, OpenZeppelin v0.3.0 and Rust 1.88.0 are pinned for
compatibility. Cargo.lock must be committed.

| Crate | Purpose |
| --- | --- |
| `remitpulse-account` | WebAuthn quorum account and bounded Ed25519 sessions |
| `remitpulse-webauthn` | ES256 WebAuthn verification, also linked into the account |
| `remitpulse-subsidy` | Administrator-authorized token subsidy vault |
| `remitpulse-router` | Atomic invocation router |
| `remitpulse-airdrop` | Merkle token distribution |
| `remitpulse-nft` | NFT contract |

`crates/webauthn-verifier` contains the shared verification library, so the account
does not export the standalone verifier endpoint.

## Build and test

Install Rust using rustup, then a Stellar CLI with Protocol 22 support (the
successor to `soroban-cli`). The toolchain file installs the required Rust target.
Deployment-script tests require Python 3.11 or later.

Run the complete local CI suite with:

```sh
make ci
```

Or run individual checks:

```sh
cargo test --workspace --locked
python3 -m unittest discover -s tests -v
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
shellcheck deploy.sh scripts/build.sh
cargo audit --file Cargo.lock
cargo build --workspace --locked --release --target wasm32v1-none
make build
```

`cargo clippy --workspace --all-targets --locked -- -D warnings` is part of the
CI quality gate (`make ci` and the GitHub Actions workflow), so any lint warning
in repository code fails the build. The pinned `clippy` component from
`rust-toolchain.toml` is installed in CI before the check runs, and dependency
lint output is excluded so the check focuses on this workspace. Exceptions are
narrowly scoped and justified inline: an `#[allow(clippy::...)]` attribute must
name the specific lint, apply to the smallest possible item, and carry a short
comment explaining why the lint cannot be satisfied. Blanket crate-level allows
are not accepted. ShellCheck covers `deploy.sh` and `scripts/build.sh`; any
suppression is scoped to a single line with an inline `# shellcheck disable=...`
comment and a short justification. `cargo audit` reads `Cargo.lock` and fails on
known advisories; install it with `cargo install cargo-audit --locked`. If an
advisory must be deferred, add a time-bounded entry to `audit.toml` with an
expiry and rationale, and remove it once the dependency is updated. CI runs all
of these checks on every push and pull request.

`make build` and `deploy.sh` use `scripts/build.sh`, which explicitly selects the
Rust version from `rust-toolchain.toml` for the CLI and its Cargo subprocesses.
This also handles shells or CI jobs with `RUSTUP_TOOLCHAIN=stable`, which would
otherwise override the repository's pin. The Stellar CLI itself may be installed
using a newer compiler; contract compilation still uses the pinned toolchain.

The initial compiler incompatibilities are resolved by Rust **1.88.0**:

- Rust 1.99.1 rejects `ethnum` 1.5.2's integer-error transmute because the standard
  library's `TryFromIntError` representation changed.
- Rust 1.85.0 cannot compile OpenZeppelin v0.3.0's `is_multiple_of` call, which was
  not yet stable in that toolchain.
- Rust 1.88.0 supports the locked dependencies and passes the native test suite
  and release WASM builds. Keep the toolchain pin and `Cargo.lock` together.

For a manual build outside the wrapper, clear an inherited override with
`env -u RUSTUP_TOOLCHAIN cargo build --workspace --locked --release --target wasm32v1-none`.
Run builds from this repository so rustup finds its toolchain file.

`stellar contract build` builds and optimizes WASM by default with current CLI
versions. Artifacts are `target/wasm32v1-none/release/remitpulse_*.wasm`.
Older `soroban` CLI releases use a separate optimization step:

```sh
soroban contract optimize \
  --wasm target/wasm32v1-none/release/remitpulse_account.wasm \
  --wasm-out target/wasm32v1-none/release/remitpulse_account.optimized.wasm
```

The CLI must support `wasm32v1-none` and constructor arguments after `--`.
The deployment script supports `STELLAR_CLI=soroban` for such legacy releases.
[Stellar build/deploy documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/hello-world).

Tests use the built-in Soroban test SDK. The extracted contracts retain their
original unit/integration suites. New tests cover session limits, expiry,
revocation, token scope, real signed custom-account authorization, SAC transfers,
replay protection, and subsidy authorization. Deployment script tests stub the CLI to verify constructor arguments, network selection,
preflight validation and build failure handling. No RPC or funded account is needed.

Run `make verify` before release. It runs the native suite, deployment checks,
format checks, builds the release artifacts, and runs the account suite again
against `remitpulse_account.wasm` inside the Soroban VM. CI uploads the exact
WASM files with a `SHA256SUMS` manifest, so release candidates can be verified
against the reviewed build output.

## CI artifacts and release verification

CI publishes the release WASM binaries as GitHub Actions run artifacts on the
workflow run for the reviewed commit. Artifacts are named
`remitpulse-contracts-<commit-sha>-<build-target>`, where `<commit-sha>` is the
full 40-character commit SHA and `<build-target>` is the build target (for
example `wasm32v1-none`). Each artifact contains the `remitpulse_*.wasm` files
and a `SHA256SUMS` manifest. Artifacts are retained for **90 days**; download
and archive the ones needed for a release before they expire.

Release operators can download an artifact from the Actions UI for the
reviewed run, or with the GitHub CLI:

```sh
gh run download <run-id> \
  --name remitpulse-contracts-<commit-sha>-wasm32v1-none \
  --dir artifacts
```

Verify the downloaded binaries against the reviewed build output using the
uploaded manifest:

```sh
cd artifacts
sha256sum -c SHA256SUMS
```

Alternatively, rebuild from the same reviewed commit with `make build` and
compare checksums against the manifest (or the downloaded files) to confirm the
artifacts match a reproducible local build. Do not deploy binaries whose
checksums do not match the reviewed output.

## Verify WASM provenance

CI uploads a `provenance.json` record alongside the WASM artifacts and
`SHA256SUMS`. The record lists, for each artifact, the commit SHA, the Rust
toolchain version, the build target (`wasm32v1-none`) and the artifact checksum.
Before deploying a downloaded artifact:

1. Recompute the checksum and compare it with both `SHA256SUMS` and the
   `checksum` field in `provenance.json`:

   ```sh
   sha256sum remitpulse_account.wasm
   grep remitpulse_account.wasm SHA256SUMS
   ```

2. Confirm the `commit` field matches the reviewed release commit
   (`git rev-parse HEAD` on the reviewed checkout).
3. Confirm the `toolchain` field matches `rust-toolchain.toml` and the `target`
   field is `wasm32v1-none`.

If any value differs, do not deploy the artifact; rebuild from the reviewed
commit with the pinned toolchain instead.

## Passkeys and sessions

Construct an account with `Config`: unique uncompressed SEC1 P-256 public keys
(65 bytes), quorum `threshold`, SHA-256 of the WebAuthn RP ID (`rp_id_hash`), and
an exact HTTPS origin encoded as bytes. Registration, attestation policy and
credential ownership verification belong in the client enrollment flow.
The constructor fixes this quorum for the lifetime of the account; this version
has no recovery, key rotation or upgrade mechanism.

Privileged operations require a `Signature::Passkeys` vector. Each assertion
contains its unique `key_index`, raw authenticator data, original client JSON and
64-byte ECDSA `r || s`. Clients must convert DER signatures to raw and normalize
S to low-S. The challenge is the unpadded base64url encoding of the **Soroban
authorization signature payload**, not an independently generated challenge.
Verification enforces RP ID hash, exact origin, `webauthn.get`, no cross-origin
ceremony, user presence and user verification. Soroban enforces authorization
nonce/ledger expiry. Authenticator sign counters are not tracked (synced
passkeys commonly report zero). JSON strings must use literal characters rather
than escaped representations for the compared fields.

1. Obtain a full passkey quorum authorizing `account.add_session(public_key, session)`.
2. Scope the session to a verified token contract address, optionally one fixed
   recipient, a positive per-transfer amount, total remaining budget and an
   absolute ledger timestamp expiry (seconds). Bind the recipient for a typical
   remittance payout; leaving it empty permits any recipient within the limits.
3. Sign subsequent Soroban authorization payloads with the session's Ed25519 key.
   Every authorized context must be that token's `transfer(account, recipient, amount)`.
   Transfers must be positive, individually under the cap and cumulatively under budget.
4. Authorize `revoke_session` with the full passkey quorum to terminate access.
   The same quorum can call `set_sessions_paused(true)` for an immediate
   account-wide session-key freeze, without preventing a passkey transfer or
   revocation. Resume with `set_sessions_paused(false)` after investigation.

Sessions cannot authorize approvals, arbitrary calls, deployments, new sessions
or revocations. A session permits any recipient within its limits; protect the
session secret on the client. Multiple transfer contexts consume their aggregate
amount. Failed transactions roll back the budget. A successful transfer may
extend account storage TTL; expired sessions never gain validity from TTL renewal.
Operators must monitor/extend contract instance and code TTL and restore archival
entries as needed. Instance storage retains budgets and revocations together.

## Multi-currency transfers

USDC, EURC and regional anchor assets such as ARST/BRLA use the same SEP-41
`transfer` interface, including Stellar Asset Contracts. Identify assets by
**network-specific contract address and verified issuer**, never ticker alone.
Configure one session key per asset; amounts are signed `i128` token base units.
Read each asset's `decimals()`; do not assume all assets have seven decimals.
Budgets are denominated in the selected token, without FX conversion or a price
oracle. RemitPulse does not mint these currencies or perform swaps. Recipients
of anchor assets may need trustlines and issuer authorization; failed transfers
revert atomically. Airdrop instances distribute one configured asset each.

## Deploy and initialize

Create and fund a CLI identity on testnet:

```sh
stellar keys generate remitpulse-deployer --network testnet --fund
export SOURCE_ACCOUNT=remitpulse-deployer
export ADMIN="$(stellar keys address remitpulse-deployer)"
```

Set `ACCOUNT_CONFIG` to JSON with real registered passkey material. Hex strings
represent byte fields; `keys` is an array of 130-character public-key hex strings,
`rp_id_hash` is 64 hex characters, and `origin` is the hex encoding of your origin.
Do not deploy with placeholder keys; there is no recovery path in this version.

```sh
export ACCOUNT_CONFIG='{"keys":["<SEC1_PUBLIC_KEY_HEX>"],"threshold":1,"rp_id_hash":"<SHA256_RP_ID_HEX>","origin":"<ORIGIN_UTF8_HEX>"}'
./deploy.sh testnet core
# For a funded mainnet identity with the same environment inputs configured:
./deploy.sh mainnet core
```

`deploy.sh` validates `ACCOUNT_CONFIG` before compilation or submission: exact
fields, threshold, duplicate keys, valid uncompressed on-curve P-256 keys,
32-byte RP ID hash, and a canonical HTTPS origin. It also validates the airdrop
root and NFT constructor input. It cannot derive the RP ID from its hash, so the
enrollment service must independently ensure the RP ID is valid for the origin.
Use the `config`, `session`, `sessions_paused`, `admin`, `is_reimbursed` and
`active_epoch` read methods for operational monitoring. Account events use
`session` topics for set, revoked, paused, and spent transitions; subsidy events
use `subsidy` plus the reimbursement ID, and epoch rotation emits an event so
operators can track the active epoch.

`core` deploys router, verifier, account and subsidy vault. Account initialization
and admin assignment happen atomically through constructors. A separate verifier
deployment is optional: account calls the same verifier code internally.
The script validates required variables, builds/optimizes before submitting,
stops on failures and records each successful ID in `deployments/<network>-*.env`.
Deployments are separate transactions; earlier deployments remain if a later one
fails. No network deployments are performed by tests or CI.

Deploy optional extracted components individually:

```sh
# Set TOKEN, ROOT_HASH, FUNDER, ADMIN to the asset address, Merkle root and owners.
./deploy.sh testnet airdrop
# Set OWNER, MAX_SUPPLY, METADATA (JSON with base_uri, name and symbol).
./deploy.sh testnet nft
```

`./deploy.sh testnet account`, `subsidy`, `router` or `webauthn` deploys just that
component. `make test`, `make build`, `make optimize`, `make deploy-testnet` and
`make deploy-mainnet` provide shortcuts. `COMPONENT=account` selects a component.

## Subsidies and operational boundaries

Fund the subsidy vault using a regular token transfer. Its administrator calls
`reimburse(id, asset, recipient, amount)` once per unique 32-byte reimbursement ID.
Only the administrator can select the asset, recipient and amount. Failed token
transfers roll back the replay marker.

Replay markers are bounded by an explicit epoch policy. The vault exposes
`active_epoch()`, which returns the current epoch number, and the administrator
can call `rotate_epoch()` to advance to the next epoch. Rotation clears the
replay markers of the previous epoch, so instance storage grows only with the
reimbursements recorded in the active epoch rather than for the vault's entire
lifetime. Replay rejection is scoped to the active epoch: a reimbursement ID is
rejected while its marker exists in the active epoch, and IDs from earlier
epochs are no longer tracked. Operators must therefore treat reimbursement IDs
as unique per epoch and choose a rotation cadence (for example, per settlement
window) that keeps the active epoch's marker count within the intended storage
budget. Rotate only after every reimbursement from the closing epoch has been
settled and reconciled; markers from a rotated epoch cannot be recovered.

This contract cannot pay Stellar transaction fees. A backend fee-bump sponsor
must submit and fund transactions, enforce fee caps and reimbursement policy,
and protect its signing key. Token reimbursement and fee sponsorship are separate
operations; the vault does not prove that a network fee was actually paid.

These contracts have not received an independent security audit. Mainnet
deployment support is provided, but production use requires review of the
account and enrollment flow.

## Interface reference

[`docs/INTERFACES.md`](docs/INTERFACES.md) is the maintained reference for every
public contract entrypoint, including authorization requirements, input
validation, storage effects, emitted events and error codes. It is checked
against the source in CI, so a contract change that alters an entrypoint, event
or error without updating the reference fails the build. Consumers should treat
it as the stable interface contract; review it alongside any contract change.

## Mainnet deployment checklist

[`docs/MAINNET_CHECKLIST.md`](docs/MAINNET_CHECKLIST.md) is the preflight
checklist for production releases. It covers commit pinning, WASM checksum
verification against the CI `SHA256SUMS` manifest, network and RPC
configuration, administrator identity confirmation, and rollback/incident
readiness, and it distinguishes testnet from mainnet requirements. Maintainers
must complete it during release review and execution before any mainnet
deployment. It never requires private keys or other sensitive values to be
committed; secrets stay in the operator's environment.

## Branch protection

The `main` branch is protected:

- Changes to `main` require a pull request; direct pushes are not permitted.
- The contract CI workflow (`make ci`, including the contract test suite) is a
  required status check that must pass before merge.
- Force pushes and branch deletion are restricted to maintainers per the
  repository's configured policy.

Documentation review is part of the pull-request expectations: changes that
touch contract entrypoints, events or errors must update
[`docs/INTERFACES.md`](docs/INTERFACES.md), and release-affecting changes must
confirm the [mainnet checklist](docs/MAINNET_CHECKLIST.md) where applicable.
