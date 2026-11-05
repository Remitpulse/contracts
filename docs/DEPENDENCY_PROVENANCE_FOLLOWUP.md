# Dependency and provenance follow-up — 2026-11-05

This is a maintainer follow-up, not production approval or independent audit
certification. No deployment, blockchain transaction, funding document, advisory
suppression or license change was made. No messages were sent to upstream authors.

## Exact repository and working trees

The requested repository is **https://github.com/Remitpulse/contracts**, not the
initial shell directory `/workspaces/backend`.

| Location | Branch / revision | Status when located |
| --- | --- | --- |
| `/tmp/remitpulse-contracts` | Local `fix/contracts-readiness`, `2deb91bfd0c9fd044abdd254e55e091766d12985` | Clean; contains previous fixes. |
| Published [PR #38](https://github.com/Remitpulse/contracts/pull/38) | `fix/contracts-readiness`, `433ac055553eb8324ad7ee4143c3f0e2624a0f4f` | Open draft, not merged; same tree as local checkout: `976063522c66031cfd5de1d9aed8bb1391d7504a`. Commit metadata differs because publication used GitHub's connected app. |
| `/tmp/remitpulse-contracts-followup` | `fix/dependency-provenance-followup`, based on published #38 head | Separate Git worktree for this follow-up. Its ignored target symlink shares the prior build cache; source trees are separate. |
| Contracts main | `f1b9b52212f4cba2c2c2735ff29cef8713f07c66` | Still the broken baseline; previous fixes are not yet on main. |

Follow-up source changes were tested at local commit
`d7575d3bab26ddb60b5a383c68e1c2c1fdeb50a5`, published as
`196d2134ec8786cc76e1e05b701716cbe4f5c93e`; their Git trees were checked identical:
`cef584bb92dff1e5be9025ff7960838c0117b325`. Documentation-only additions follow
that source snapshot. The follow-up PR targets `fix/contracts-readiness` so its
diff does not duplicate #38. Do not merge either PR automatically.

## Dependency investigation and safe change

Actual dependency paths:

```text
soroban-sdk 22.0.11
  -> soroban-env-host =22.1.3 (native/testing)
     -> p256 0.13.2 / k256 0.13.4 / elliptic-curve 0.13.8
        -> crypto-bigint ^0.5 (locked 0.5.5)
account / standalone WebAuthn dev-dependencies -> p256 0.13.2 -> same path
```

The SDK also pins guest/common components exactly; this is not a freely updatable
host package. The current registry contains host 22.1.4, but SDK 22.0.11 requires
**=22.1.3**, and host 22.1.4 still requires elliptic-curve 0.13 / p256 0.13 / k256
0.13. Substituting it would neither satisfy the SDK nor repair this dependency.

[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)
describes timing variability in operations intended to be constant-time, with
patched releases >=0.7.6. A refreshed registry has no patched 0.5 release: the
latest 0.5 version is 0.5.5. Version 0.7.6 is now yanked; the available non-yanked
0.7.10 is also outside the required ^0.5 range. Both resolution attempts exited
**101**, without changing the lockfile:

```sh
cargo update -p crypto-bigint --precise 0.7.6
cargo update --offline -p crypto-bigint --precise 0.7.10
# failed to select a version for crypto-bigint = "^0.5"
# required by elliptic-curve v0.13.8 -> p256 v0.13.2
```

`cargo update --dry-run` refreshed registry/git metadata and proposed only the
SDK host's broad Dalek dependency moving to incompatible 3.x plus its transitive
packages. That preview was not applied; the prior review reproduced the host's
rand_core mismatch with that major. No compatible security version update was
available in this graph, so no crate version was changed and no patched-number
substitution, new fork, vendored crypto backport or advisory ignore was introduced.
A coordinated SDK/OpenZeppelin/crypto migration or a reviewed upstream backport is
needed; merely updating p256's test dependency leaves the SDK host path in place.

The safe dependency change is **source pinning**: all six OpenZeppelin workspace
dependencies now use full revision `659fcabdc0e4712254dfebe4f45efd54f49826e9`
instead of the v0.3.0 tag. All **212 package name/version pairs** are unchanged;
the eight resolved OpenZeppelin package source URLs now name that exact revision.
The lockfile already protected locked builds; the manifest now also fixes the
source selected when regenerating a lockfile. This is not advisory remediation.

The normal `wasm32v1-none` dependency graph has no crypto-bigint path. The native
SDK host and test fixture signers do; contract signature verification imports
host cryptography and uses public inputs. This narrows observed exposure, but is
not an exploitability proof or accepted exception. The gate remains red.

## Provenance and license evidence

| Component | Exact evidence | Conclusion |
| --- | --- | --- |
| Router | Creit-Tech main/source revision `04975c434dec362584aa99458ae2c25d803ec570`, `contracts/router-v0/src/lib.rs`, Git blob `0e4a95203b7d15cfc273df0f231f248eb22bb834` | Caller auth and invocation loop predate RemitPulse. No LICENSE/COPYING/NOTICE history or first-party license declaration was found in the inspected tracked text/history. No redistribution permission is inferred. |
| Historical Merkle test proof | First matching committed root/recipient/proof found in philipliu source revision `c01829c0b2d3e441bd4d9da431d06ea42c675d31`, `airdrop/contracts/airdrop/src/test.rs`, blob `75f839ab1a04b05781458bb8f104ffcde457bbaa` | Resolves a specific matching upstream fixture source, not proof of the original generation execution. |
| Full four-recipient input | philipliu `b611ea8d017a7fe1a9c6e4386e8491c16c821fce`, `merkle/receivers.json`, blob `26eb60700a57ca7d0d9ee1e3c5c03a4e58b16ea5` | These inputs reproduce the historical root and index-3 proof in the new local test. |
| Upstream generator | philipliu main `0c183ed551156e264e80e9345cc0fa55f06d32e0`; `merkle/src/main.rs` blob `32d90cda9f112a90dd7e62bdff7662c6bea8307a`, also present at c01829c | Source serializes indexed receiver maps as XDR and hashes/sorts leaves/pairs. It was inspected, not vendored or executed. No repository license/first-party grant was found. |
| Immediate OpenZeppelin example | `857bd364a88c4e50faae8b2c160d91b108f1c53d`, `examples/fungible-merkle-airdrop/src/test.rs`, blob `997660fe213dc7f374800b3574642d727d21a318` | Contains the same root, recipient, amount and index-3 proof under the repository's MIT notice. The source is attributed; upstream authorship/grant for earlier fixture generation is not independently established. |

Links: [router source](https://github.com/Creit-Tech/Stellar-Router-Contract/blob/04975c434dec362584aa99458ae2c25d803ec570/contracts/router-v0/src/lib.rs),
[matching Merkle test](https://github.com/philipliu/soroban-merkle-airdrop/blob/c01829c0b2d3e441bd4d9da431d06ea42c675d31/airdrop/contracts/airdrop/src/test.rs),
[recipient inputs](https://github.com/philipliu/soroban-merkle-airdrop/blob/b611ea8d017a7fe1a9c6e4386e8491c16c821fce/merkle/receivers.json),
[OpenZeppelin test](https://github.com/OpenZeppelin/stellar-contracts/blob/857bd364a88c4e50faae8b2c160d91b108f1c53d/examples/fungible-merkle-airdrop/src/test.rs).

The new test reconstructs the four XDR leaves with the contract's own Receiver
schema and SDK SHA-256, checks the exact historical root and proof, then exercises
real local token transfers. Changed index/recipient/amount are rejected before a
valid claim; replay is rejected afterward. No upstream generator implementation
or entire JSON proof file is copied into this repository. The upstream JSON's
index-0 proof includes repeated entries; it is not treated as a trusted wholesale
fixture import. Reproducibility does not settle copyright/licensing questions.

Existing source headers remain. `LICENSE` and `LICENSES/OpenZeppelin-MIT.txt` are
unchanged (SHA-256 respectively
`81441819e11af4edb09adf44ecbc8638097f3c800932ff7b4a026c4aa8082187` and
`7414c0b6f573947bb42dc8c547f79a131d80e37bb96b16d13e9a34f0d53af905`).
This is evidence collection, not legal advice or a new grant of license terms.

## Compatibility and responsibility split

[The detailed matrix](COMPATIBILITY_REVIEW.md) pins the latest frontend/backend
main SHAs and compares 18 contract/auth areas and seven HTTP flows. The heads are
unchanged from the prior review, but their source was rechecked. No consumer
adapter is silently introduced and no authorization boundary is relaxed.

| Work | Status / responsible decision |
| --- | --- |
| Freeze current upstream source; reconstruct fixture; refresh matrices and checks | Safely completed in this follow-up; no approval needed for these reversible changes. |
| Fix crypto advisory / abandoned transitive packages | No drop-in update exists. Maintainers must select and review an upstream backport or coordinated SDK/OpenZeppelin/crypto migration, including protocol, Rust and ABI implications. No exception is accepted here. |
| Factory/enrollment, quorum key mapping, session units/scopes, subsidy and HTTP protocol | Product/security maintainers must agree the versioned contract and API before cross-repository implementation and integration acceptance. Do not deploy funded wallets using current clients. |
| Router and historical fixture redistribution rights | Obtain a documented grant/clarification from the relevant rights holders or approve a separately scoped replacement. Repository owners cannot invent an upstream license. No notices are removed as a workaround. |
| Branch protection and private vulnerability intake | Repository administrators must configure and verify these settings; this follow-up does not change them. |
| Deployment evidence and artifact identity | Maintainers must supply existing Testnet IDs/transactions/WASM hashes and pin CLI/optimizer plus exact-artifact checksum policy. Deployment/transaction submission remains outside this task. |
| Merge/release | Human review only. This PR is stacked on #38, the audit is failing, and neither production readiness nor certification is claimed. |

## Re-run results

Toolchain: Rust 1.88.0, SDK 22.0.11, cargo-audit 0.22.2, ShellCheck 0.11.0.
RustSec database: `26fd2d43f7280a1fb925a3743ec463017afe7119`, 1409 advisories,
212 scanned dependencies. No ignores or target filters were used for the audit.

| Check | Exit / result |
| --- | --- |
| `cargo check --workspace --offline` while resolving immutable sources | 0; versions unchanged. |
| `cargo audit --file Cargo.lock --json` | **1**; RUSTSEC-2026-0387 / crypto-bigint 0.5.5, plus two unmaintained warnings. |
| `make ci` | **2**, audit gate failure; not a passing complete CI result. Earlier formatting, native tests, all Clippy variants and ShellCheck completed successfully. |
| `cargo test --workspace --locked` inside make ci | **70 passed, 0 failed, 0 ignored**: account 10, airdrop 13, NFT 31, router 6, subsidy 7, WebAuthn 3; no additional shared-library/doctests. |
| `python3 -m unittest discover -s tests -v` independently after gate failure | **0 / 18 passed**; CLI stubs only. |
| Formatting / all-target Clippy with `-D warnings` / ShellCheck / `bash -n` | 0 / passed; no new suppression. |
| `make test-wasm checksums reproducible-build` independently after gate failure | **0**; six release WASM builds, account WASM suite **10 passed, 0 failed/ignored**, all checksum checks, separate target-directory builds matching for all six. |
| `sha256sum -c SHA256SUMS` at artifact root | 0 / all six OK. |

Unmaintained warnings: derivative 2.2.0 (RUSTSEC-2024-0388), paste 1.0.15
(RUSTSEC-2024-0436), inherited from native-host arkworks/VM dependencies. They
were not removed by substituting incompatible macros or hidden via ignores.

Raw Cargo release WASM manifest (not optimized CLI output or on-chain evidence):

```text
0414f69e26cfeac56ebbac985e2ec1d06acc84b365272c879751949811575307  remitpulse_account.wasm
12cc39b67d121b398cb1ca8382dc47b3992313c6479054a70b6d82368996db1d  remitpulse_airdrop.wasm
2dbd4c2706f3fc032614d741de3b64497d97f7898c095251a776e17834c8fab0  remitpulse_nft.wasm
f5eecda1bb38413ac9ed3239ad3ebc8d7779842e9c189c688626aa409c950785  remitpulse_router.wasm
8168331cf13a2a33cf6effbd9fbd80ebd5f88736ece9449872222a360b9993e4  remitpulse_subsidy.wasm
2df78010a587658e575d39d84793654b3542ad9c0978d892fffd66b97440d31e  remitpulse_webauthn.wasm
```

The airdrop/NFT hashes changed after replacing the Git source locator with the
explicit revision, despite unchanged upstream code/version. Use this manifest,
not the prior review's two old hashes; the other four are unchanged. No assertion
is made that the new binary matches any deployed contract.
