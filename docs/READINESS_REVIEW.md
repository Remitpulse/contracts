# Maintainer technical and security review — 2026-11-05

The [follow-up](DEPENDENCY_PROVENANCE_FOLLOWUP.md) records the refreshed
dependency resolution, fixture reconstruction and 70-test result. The 69-test
results and hashes below describe the earlier source snapshot.

This is a source review and local verification, not an independent security
audit, certification, or production approval. No deployment or blockchain
transaction was performed. No license terms were changed and no advisories were
suppressed. Repository settings were inspected, not changed.

## Scope and revisions

- Contracts main: `f1b9b52212f4cba2c2c2735ff29cef8713f07c66`.
- Frontend main: `7319d4e3cc4a27a477c8718eb34fd865cfff8339`.
- Backend main: `b617e2fa08d3b0020d0d1dd967c0263a03c121ba`.
- Reviewed all six contracts and shared verifier, manifests/lockfile, deployment
  scripts, native tests, CI, README/security/interface and provenance documents.
- Contracts main was confirmed with GitHub's branch API; frontend/backend main
  SHAs were independently confirmed through that API after cloning.

## Baseline evidence

The native suite could not start: Cargo.lock had a missing `[[package]]` before
`ctutils`, producing a duplicate TOML `name` error at line 410. The manifest
selected SDK 25.3.1 while OpenZeppelin v0.3.0 requires SDK 22.x, creating distinct
Env/Address types. New crypto test majors also broke compatibility with the SDK
host's broad Ed25519 dependency requirement. Restoring a coherent dependency graph
exposed additional compilation errors in NFT storage keys, pagination tests,
constructor tests, and airdrop imports/error assertions.

The original Python suite ran 15 tests and reported **10 failures and 38 subtest
errors**. Several purported valid StrKeys were invalid, validator tests invoked
a non-executable file without an argument, and the CLI stub did not produce the
artifacts the wrapper now requires. Regression fixtures now use actual checksummed
StrKeys and run copied scripts in temporary directories, isolated from real WASM.

[Main CI run 41455099502](https://github.com/Remitpulse/contracts/actions/runs/41455099502)
failed all Rust jobs while installing nonexistent Rust 1.120.0. ShellCheck passed.
The artifacts API returned `total_count: 0`. The branch API returned
`protected: false`; README/CONTRIBUTING claims of enforced protection were false.

## Findings and improvements

| Impact | Finding | Resolution |
| --- | --- | --- |
| Release blocker | Malformed lockfile, incompatible SDK/crypto graph, source and test compilation failures, unusable CI compiler pin | Restore SDK 22.0.11 consistently with OZ v0.3.0, p256 0.13.2, Dalek 2.2.0 and Rust 1.88.0; regenerate lock; fix actual storage types and test interfaces. SDK 25 migration remains separate work. |
| Medium, vault availability | Epoch rotation only changed the namespace; every previous marker stayed in instance storage forever | Index active IDs, cap epochs at 128 payments, remove markers on rotation, emit rotation event, return explicit overflow/capacity errors. New-instance layout; no live migration claimed. Test real storage deletion, capacity, authorization and downstream rollback. |
| Medium, resource policy | Router compared argument count with a byte limit, allowing a single arbitrarily large value through | Measure argument vector XDR bytes before dispatch; retain invocation cap and atomicity tests. This does not bound arbitrary target work/results. |
| Medium, NFT indexing | Token-ID cursor pagination assumed sorted IDs but insertion appended IDs, so out-of-order mints/transfers skipped tokens | Insert sorted IDs; test mint 30, mint 10, transfer 20 and complete pagination. Avoid renewing nonexistent owner-list entries, which made empty queries trap. Full owner vectors remain unbounded. |
| Low, distribution policy | Airdrop claims remained possible at/after the stated immutable deadline until admin recovery | Reject claims at the exact deadline with ClaimWindowClosed; regression checks the boundary. Recovery remains admin-authenticated to fixed funder. |
| Release blocker | Deployment preflight rejected standalone components, required irrelevant core inputs, rejected valid StrKeys (`-1 in bytes` raises), and omitted airdrop deadline | Validate inputs by component; correct StrKey checks; enforce u32 deadline and account RP/origin bounds; forward deadline; test all deployment paths with a stub. No network actions. |
| Release evidence | CI manifest included repository-relative paths unusable at artifact root; README described nonexistent provenance fields and wrong retention | Emit basename paths, document actual manifest hash and metadata fields, correct retention to 30 days. Metadata is unsigned. |
| Reproducibility | Make target passed unused OUT_DIR, reused default Cargo output and constructed invalid comparison paths | Build into two distinct Cargo target directories and compare basename manifests. |
| Open-source blocker | Root MIT notice alone did not preserve OZ notice; router source has no identified license | Preserve exact OZ MIT notice in LICENSES; document source revisions and unresolved router/fixture rights. Root LICENSE unchanged. |

## Authorization and cryptography assessment

The account verifies unique key indices against an immutable quorum, RP hash,
exact origin/type/challenge, non-cross-origin ceremony, UP and UV, and ES256 via
the Soroban host. Challenges use the host's network/nonce/invocation/expiration
bound signature payload. JSON is bounded and fully consumed; duplicate known
fields and crossOrigin=true are tested using freshly signed malformed data.
Authenticator counters and credential IDs are not stored; the enrollment system
must map credentials to configured key indices, enforce RP-origin relationships,
verify possession/attestation policy, convert DER to raw low-S, and collect quorum.
The standalone verifier has caller-supplied trust inputs and no replay state; its
success is not an authorization grant.

Sessions check timestamp expiry (including equality), revocation/pause, exact
token address, transfer method/arity/from-address, optional recipient and positive
amounts. Checked addition covers aggregate budget overflow; signing happens over
the host payload. Session signatures cannot authorize administration, arbitrary
methods, approvals or deployment contexts. Tests cover real SAC transfers and
reused nonce rejection, aggregate overflow/overspend, state preservation after
failure, unauthorized mutations, token restrictions and recipient binding.

Subsidy transfers/rotation require stored admin auth. The admin deliberately can
select any asset/recipient; this is not a permissionless proof of fee payment.
Markers are written before token transfer and roll back with the ID index if a
downstream token rejects. IDs become reusable after rotation; reconciliation and
admin-signature expiry across epoch boundaries remain operational responsibilities.
Checked increment prevents epoch wrap; release overflow checks remain enabled.

Router caller auth does not replace downstream token/contract authorization.
Airdrop recipients authorize their own indexed proof claims; root/token/funder
and recovery deadline are immutable. NFT token auth and logical approval expiry
are inherited from pinned OZ Base. Local code no longer applies persistent TTL
operations to temporary approval-for-all keys. Lifetime minted supply remains
monotonic across burns and is documented as such.

The SDK 22.0.11 pin also includes Stellar's published patches for
[range-bound overflow (GHSA-96xm-fv9w-pf3f)](https://github.com/stellar/rs-soroban-sdk/security/advisories/GHSA-96xm-fv9w-pf3f)
and [Fr equality reduction (GHSA-x2hw-px52-wp4m)](https://github.com/stellar/rs-soroban-sdk/security/advisories/GHSA-x2hw-px52-wp4m).
This repository does not use Fr comparisons in its authorization logic.
These vendor advisory checks supplement RustSec; a successful cargo-audit scan
is not an exhaustive inventory of all vendor vulnerabilities.

## Dependency audit — unresolved

`cargo-audit` **0.22.2** scanned **212** locked dependencies against **1409**
RustSec advisories, database commit `26fd2d43f7280a1fb925a3743ec463017afe7119`.
It reported **1 vulnerability and 2 unmaintained warnings**, with no ignored IDs:

- [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387):
  crypto-bigint 0.5.5, timing variability in operations intended to be
  constant-time; fixed in >=0.7.6. Dependency path is SDK native host and
  p256/k256 -> elliptic-curve 0.13.8 -> crypto-bigint 0.5.5.
- RUSTSEC-2024-0388: derivative 2.2.0 unmaintained, via native-host arkworks.
- RUSTSEC-2024-0436: paste 1.0.15 unmaintained, via arkworks/wasmi.

`cargo tree --locked --target wasm32v1-none --edges normal -i crypto-bigint`
prints no dependency path. Native tests use fixed, public fixture signing keys;
contract verification inputs are public and deployed WASM imports host crypto.
This is an exposure assessment, not proof of universal non-exploitability or
an accepted exception. Upgrading crypto-bigint across its incompatible API
versions needs a compatible upstream SDK/crypto upgrade or reviewed backport;
a lockfile-only replacement is not a safe fix. No ignores, advisory database
rollback or blanket suppression were added. `cargo audit` exits 1 and `make ci`
exits 2 at the audit gate. Keep that failure visible until remediation or a
separately justified, approved exception is implemented.

## Frontend/backend compatibility — blocked

| Consumer | Current assumption | Contract implementation |
| --- | --- | --- |
| Frontend `lib/stellar/signatures.ts` | `WebAuthn({id,...})`, `Ed25519({public_key,signature})` | `Passkeys(Vec<Assertion {key_index,...}>)`, `Session(public_key,signature)` tuple enum |
| Backend `src/interfaces/contracts/bindings.ts` | Passkey/session signatures as bare maps | Tagged tuple enums above; passkeys require quorum vector and key indices |
| Backend wallet factory bindings/service | `deploy(salt,key_id,public_key)` and `get_address(salt)` on ACCOUNT_CONTRACT_ID | No factory entrypoints. Deploy account WASM with full Config constructor. |
| Backend session bindings | `add_session_key(key,token,perTx,total,expiresAtLedger)` and `revoke_session_key` | `add_session(key,Session)` and `revoke_session`; expiry is u64 Unix seconds, not u32 ledger |
| Backend subsidy bindings | `allowance(account)` / `claim(account)` | Admin-only `reimburse(id,asset,recipient,amount)`; no allowance/claim API |

Frontend DER-to-compact-low-S conversion is aligned in intent. SEP-41 transfer
arguments align. These agreements do not repair the overall ABI. No cross-repo
code was changed; implementing missing factory or adapting consumer enrollment,
quorum signing and subsidy policy requires separate coordinated changes and
end-to-end integration tests. Do not fund wallets through the current app flows.

## Deployment and Testnet evidence

There are no committed Testnet contract IDs, transaction hashes, deployment
manifests or reviewed WASM checksums that link a deployment to this source.
The current-main CI run has no artifacts. Therefore deployed-code identity,
constructor state and successful Testnet flows cannot be independently verified
from this repository. No transaction or deployment was submitted to fill that gap.
The deployment script tests are local CLI stubs and are not Testnet evidence.
A real Stellar CLI build/optimizer was not installed or executed in this review:
release builds use Cargo directly. The wrapper still calls an unpinned CLI that
may optimize, and deploy.sh rebuilds instead of consuming a reviewed artifact;
it does not enforce a supplied checksum. Raw Cargo CI hashes therefore do not
prove the identity of an optimized/deployed binary. Pin the CLI/optimizer and
establish a reviewed-artifact deployment/checksum gate before release.
Release WASM checksums from this review are recorded below after local builds.

## Remaining release blockers and limitations

- Remediate RUSTSEC-2026-0387 and review the two unmaintained dependencies; the
  dependency audit is red and no exception is accepted.
- Align frontend/backend ABIs and enrollment/quorum flows; exercise complete flows
  against reviewed release WASM and then obtain maintainer-supplied Testnet evidence.
- Resolve router upstream licensing and exact Merkle fixture provenance before
  calling the repository fully cleared for open-source redistribution.
- Pin CLI/optimizer versions and gate deployment on the exact reviewed artifact
  checksum; raw Cargo hashes cannot attest to the wrapper's rebuilt output.
- Enable actual branch protection/review/status checks and confirm private security
  reporting availability. Neither is established by Markdown claims.
- Account constructor validates key prefix/field range but not curve membership or
  ownership; preflight checks curve membership. On-chain origin validation is not
  full canonical URL validation. Independently validate enrollment before funding.
- No account recovery/rotation/upgrade and no subsidy admin rotation. Session count
  and NFT owner vectors are unbounded; quorum/admin users can hit storage limits.
- Restore archived code/instance/persistent entries consistently. TTL renewal is
  not archival restoration. NFT metadata/indexing and OZ state can archive apart.
- Subsidy epoch layout is for new instances; separately review migration and fund
  movement for any existing vaults. Admin policy must reconcile IDs before rotation.
- Complete a separately scoped SDK/protocol modernization before claiming latest
  SDK support; this change restores the compatible SDK 22 deployment line.

## Final local verification

Code tested: local commit `4e6007e66b6bb0de9ad86cf9e09b426fe641a415`.
Published source commit: `08c61876331335ea82cb62ed615d4c51876f6a2f`.
Their Git tree IDs were verified identical:
`22de21b11b236b8b4aaf04fdfaa34497664e1811`. Publication used the connected
GitHub app because the CLI Git credential lacked push access; only commit
metadata differs between those source commits.
Documentation-only follow-up changes do not alter contract artifacts.
Compiler: rustc 1.88.0 (6b00bc388 2025-06-23), target wasm32v1-none.

| Check | Exact result |
| --- | --- |
| `cargo test --workspace --locked --no-fail-fast` and final standard workspace run inside make ci | 69 passed, 0 failed, 0 ignored: account 10, airdrop 12, NFT 31, router 6, subsidy 7, standalone WebAuthn 3. Shared verifier and enabled doctests: 0 additional tests. |
| `python3 -m unittest discover -s tests -v` | 18 passed; all CLI calls stubbed and tooling copied to temporary directories. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, no warning suppression added. make ci also completed its additional lint/check variants. |
| ShellCheck 0.11.0 and `bash -n deploy.sh scripts/build.sh` | Passed. |
| `cargo build --workspace --locked --release --target wasm32v1-none` | Passed, 6 contract WASM files. |
| `cargo test -p remitpulse-account --locked --features wasm-tests` | 10 passed, 0 failed, 0 ignored; release-WASM feature exercises the account authorization flows in the Soroban VM. |
| `make reproducible-build` | Passed: distinct Cargo target directories, all 6 artifact SHA-256 values identical. |
| cargo-audit 0.22.2 / `cargo audit --file Cargo.lock` | FAILED (exit 1): 1 vulnerability, 2 unmaintained warnings, 0 ignores. |
| Complete `make ci` | FAILED (exit 2) at the audit gate. Remaining artifact checks were run independently with `make test-wasm checksums reproducible-build`. |

These are **raw Cargo release WASM** hashes, not CLI-optimized artifacts or
on-chain deployment evidence:

```text
0414f69e26cfeac56ebbac985e2ec1d06acc84b365272c879751949811575307  remitpulse_account.wasm
ddbb193dea497932acbeb61424268be6d8721385882dbfe5742f989a1f82e2b2  remitpulse_airdrop.wasm
be042449d18953c6350d9d0c763c2827221572d256ad7a3486037f5fcc64fb67  remitpulse_nft.wasm
f5eecda1bb38413ac9ed3239ad3ebc8d7779842e9c189c688626aa409c950785  remitpulse_router.wasm
8168331cf13a2a33cf6effbd9fbd80ebd5f88736ece9449872222a360b9993e4  remitpulse_subsidy.wasm
2df78010a587658e575d39d84793654b3542ad9c0978d892fffd66b97440d31e  remitpulse_webauthn.wasm
```
