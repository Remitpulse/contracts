# Historical maintainer source review

Current results supersede this earlier review: see [2026-11-05 readiness review](READINESS_REVIEW.md).

## Scope

Source review of workspace configuration, dependencies, every contract,
WebAuthn/custom-account auth, deployment tooling, tests, CI, Dependabot, and
documentation. This is not an independent audit or Mainnet approval.

## Fixed findings

| Severity | Finding | Evidence | Fix |
| --- | --- | --- | --- |
| Medium | Underfunded subsidy requests exposed downstream token failures. | `reimburse` transferred without a balance precheck. | Added explicit `InsufficientBalance` before the marker write and test coverage. |
| Medium | Mainnet deployment had no positive confirmation and aliases were not intent-bound. | `deploy.sh` accepted `mainnet` directly. | Require canonical passphrase and `REMITPULSE_ALLOW_MAINNET=yes`; write manifest metadata and test guards. |

## Verified controls

- Assertions enforce RP ID, origin, ceremony, challenge, ES256 signature, UP/UV.
- Sessions allow only positive, scoped SEP-41 transfers within aggregate budget.
- Soroban authorization nonce/expiry and network-bound payload provide replay resistance.
- Subsidies require administrator auth and unique IDs; failed calls roll back state.
- Router batches are caller-authorized and atomic. Airdrop recovery is admin-only.

## Residual risks

- No recovery, rotation, upgrade path, or sign-counter tracking exists; quorum loss can lock an account.
- Direct deployment bypassing preflight can install malformed P-256 key material and create an unusable immutable account.
- Subsidy eligibility, fee caps, and actual fee-bump sponsorship are backend responsibilities.
- Router is intentionally generic, not an allowlisted transaction policy engine.
- Subsidy replay markers grow; instance TTL and archival restoration require operations monitoring. Core deployment is not atomic across contracts.

Before production, obtain an independent review, verify deployed WASM hashes,
use hardware-backed administration, and threat-model recovery before implementing it.
