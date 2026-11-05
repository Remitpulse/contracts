# Security policy

Report a suspected vulnerability privately using [GitHub private security
advisories](https://github.com/Remitpulse/contracts/security/advisories/new).
Do not open a public issue with exploit steps, keys, transaction payloads, or
an affected contract ID. Include the network, contract ID and WASM hash, a
minimal reproduction, impact, and suggested mitigations. Acknowledgment and
coordination occur through the private channel.

## Mainnet deployment checklist

Before a mainnet deployment, complete the preflight checklist in
[`docs/mainnet-deployment-checklist.md`](docs/mainnet-deployment-checklist.md). It is the
authoritative, reviewable record for a release and covers:

- **Commit pinning** — the reviewed Git commit, Stellar CLI version, Rust
  toolchain, and dependency lockfile are pinned and recorded.
- **WASM checksum verification** — release WASM hashes are recorded and the CI
  artifact's `SHA256SUMS` is verified locally against the pinned commit.
- **Network configuration** — the target network, RPC/Horizon endpoints,
  passphrase, WebAuthn RP ID, and origin are confirmed for mainnet.
- **Administrator identity** — a dedicated hardware-backed administrator is
  used, with a realistic passkey quorum verified before funding.
- **Rollback and incident readiness** — an incident runbook exists for freezing
  and revoking session keys, and a rollback plan is documented.

Testnet deployments require only the commit pinning and network configuration
sections; checksum verification, administrator identity, and rollback readiness
are mainnet requirements. Record checklist completion in the change review and
release execution, not in this repository's tracked files.

Do not commit private keys, seed phrases, passkey material, or other sensitive
values. Reference secrets by their external location (for example, a hardware
wallet or secret manager) and keep fee-bump sponsorship policy and keys outside
this repository.

The current account has no on-chain recovery, key rotation, or upgrade path.
Use a realistic passkey quorum and verify account configuration before funding.
The subsidy vault authorizes token reimbursements; it does not sponsor Stellar
fees. Keep fee-bump sponsorship policy and keys outside this repository.

## Dependency vulnerability scanning

Dependabot proposes dependency updates but is not a complete vulnerability
gate. CI additionally runs a lockfile-based advisory scan that consumes
`Cargo.lock` and fails the build when a known advisory affects a locked
dependency.

### Scan command

The scan uses [`cargo-audit`](https://github.com/rustsec/rustsec) against the
committed `Cargo.lock`:

```sh
cargo audit --file Cargo.lock
```

CI runs this command in the `audit` job. Because the scan reads
`Cargo.lock`, results are reproducible for a given commit: the same lockfile
yields the same advisory set (modulo updates to the RustSec advisory database).

### Running locally

Install the audit tool and run the scan from the repository root:

```sh
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --file Cargo.lock
```

A non-zero exit status indicates one or more known advisories affect the
lockfile. Resolve them by updating the affected dependency (and committing the
updated `Cargo.lock`), or by recording a time-bounded exception as described
below.

### Documented, time-bounded exceptions

An advisory may be accepted only when it is not exploitable in this project's
usage and no fixed release is available. Every accepted advisory must be
recorded in this section with:

- **Advisory ID** — the RustSec advisory identifier (for example `RUSTSEC-0000-0000`).
- **Justification** — why the advisory does not affect RemitPulse, including
  the affected code path and why it is unreachable or mitigated.
- **Expiry date** — the date by which the exception must be re-reviewed. The
  expiry must be no more than 90 days from the date the exception is recorded.

No exceptions are configured. Adding `--ignore` alone does not enforce an
expiry date: any future exception needs documented justification, an owner,
and an actual CI expiry check before it is accepted.

| Advisory ID | Justification | Expiry date |
| ----------- | ------------- | ----------- |
| _(none currently accepted)_ | | |

When an exception expires, re-evaluate the advisory: update the dependency if a
fix exists, otherwise renew the exception with an updated justification and a
new expiry date no more than 90 days out.

## Known dependency findings — 2026-11-05

`cargo-audit` 0.22.2 reports **RUSTSEC-2026-0387** against `crypto-bigint`
0.5.5 (timing variability in purported constant-time arithmetic). The fixed
release is >=0.7.6. It is pulled in by `elliptic-curve` 0.13.8 through the native
SDK host and p256/k256; test fixtures also sign with p256. The normal
`wasm32v1-none` dependency graph does not contain this crate. Contract signature
verification uses public data and host cryptographic imports; fixture signing
uses known test keys. This narrows observed exposure, but is not an exception
or a claim that every native consumer is unaffected.

The required upstream crypto API upgrade or reviewed backport remains open.
The audit gate intentionally **fails**; there is no ignore/suppression configured.
Two additional unmaintained warnings remain: `derivative` 2.2.0
(RUSTSEC-2024-0388) and `paste` 1.0.15 (RUSTSEC-2024-0436), inherited from the
SDK host's arithmetic/VM dependencies. See the dated review for scan evidence.

## Current release limitations

See [the dated maintainer review](docs/READINESS_REVIEW.md). No independent
security audit has been performed. Current application main branches encode
incompatible account signatures/session calls and assume a factory and subsidy
allowance endpoints that do not exist here. Do not fund an account through those
flows until the ABIs and enrollment model are aligned and tested together.

The immutable account has no recovery, rotation or upgrade path. Constructor
key checks establish prefix/coordinate range, not P-256 curve membership or
possession; deployment preflight checks the curve. Origin/RP correctness and
credential-ID-to-key-index mapping are enrollment responsibilities. WebAuthn
sign counters are not stored; replay prevention relies on Soroban auth nonces
and signature-expiration ledgers. The standalone verifier is stateless and is
not itself a replay-protected authorization endpoint.

Session state has no fixed count cap; quorum-authorized additions can fill
instance storage. Token restrictions trust the configured token implementation.
The subsidy administrator can pay arbitrary recipients/assets and reuse IDs
across epochs; reimbursement does not prove a fee was paid. A new epoch is
limited to 128 payments and clears its replay records on rotation. No existing
vault migration is included. NFT owner lists remain unbounded even with paged
responses; metadata and lists can archive separately from upstream token state.
Maintain and restore storage consistently before use.

Main is currently unprotected according to GitHub's branch API. Router upstream
redistribution permission and Merkle fixture provenance are unresolved; retain
all existing notices and the OpenZeppelin MIT notice. Private vulnerability
reporting availability must be confirmed by repository administrators; if the
advisory link is unavailable, do not publish an exploit in a public issue.
