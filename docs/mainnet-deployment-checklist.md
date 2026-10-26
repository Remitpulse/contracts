# Mainnet Deployment Checklist

Use this checklist during change review and again during release execution. It is intentionally concise: every item must be verifiable from the repository, CI artifacts, or deployment records — never from secrets committed to the repo.

> **Never commit private keys, seed phrases, mnemonics, API tokens, RPC credentials, or any other sensitive values.** Reference secrets only by name (e.g. an environment variable or secret-manager key). If a checklist item cannot be satisfied without committing a secret, stop and escalate.

## Scope: testnet vs. mainnet

| Requirement | Testnet | Mainnet |
| --- | --- | --- |
| Commit pinned to a tagged release | Recommended | **Required** |
| WASM checksum verified against CI `SHA256SUMS` | Recommended | **Required** |
| Network configuration reviewed by a second person | Optional | **Required** |
| Administrator identity confirmed out-of-band | Optional | **Required** |
| Rollback plan documented | Recommended | **Required** |
| Incident contacts and escalation path recorded | Recommended | **Required** |

Items marked **Required** for mainnet are release blockers. Testnet deployments may use a lighter process but should follow the same structure so the mainnet path is exercised regularly.

## 1. Commit pinning

- [ ] The deployment targets a specific commit SHA, not a branch or `HEAD`.
- [ ] The commit is reachable from a tagged release (mainnet: tag **required**).
- [ ] The commit SHA recorded in the release notes matches the SHA checked out for the build.
- [ ] No uncommitted local changes exist in the build workspace (`git status` is clean).
- [ ] The build was produced from the pinned commit by CI, not from a developer machine (mainnet: **required**).

## 2. WASM checksum verification

- [ ] The WASM artifact was downloaded from the CI run for the pinned commit.
- [ ] The artifact's SHA-256 matches the entry in the CI-published `SHA256SUMS` file.
- [ ] `SHA256SUMS` itself was retrieved from the same CI run and its provenance is recorded.
- [ ] The checksum was verified independently by a second person (mainnet: **required**).
- [ ] The verified checksum is recorded in the deployment log alongside the commit SHA.

Verification command (adjust the artifact name):

```sh
sha256sum --check SHA256SUMS
```

## 3. Network configuration

- [ ] The target network (chain ID / network passphrase) is explicitly set and matches the intended environment.
- [ ] RPC / Horizon endpoints point to the intended network and are reachable from the deployment host.
- [ ] Contract IDs and administrator addresses match the intended network.
- [ ] Configuration is supplied via environment or secret manager references — **no credentials in the repo**.
- [ ] Configuration was reviewed by a second person (mainnet: **required**).

## 4. Administrator identity

- [ ] The administrator address is confirmed out-of-band against the expected value.
- [ ] The administrator key is held in the approved custody solution (hardware wallet, KMS, multisig) — never in the repo.
- [ ] Required signers / thresholds are confirmed for the target network.
- [ ] Ownership or admin-transfer steps (if any) are scripted and reviewed before execution.

## 5. Airdrop claim window and recovery schedule

- [ ] The airdrop `CLAIM_DEADLINE` (claim window end) is recorded in the deployment log alongside the commit SHA and WASM checksum.
- [ ] The `CLAIM_DEADLINE` was reviewed out-of-band by a second person (mainnet: **required**) and matches the schedule published to recipients.
- [ ] The recovery policy is confirmed: recovery is rejected before `CLAIM_DEADLINE`, and the finalization event emitted at deployment is captured in the deployment log.
- [ ] The recipient-visible recovery schedule (claim deadline and post-deadline recovery) is published before traffic is enabled.
- [ ] The `CLAIM_DEADLINE` value is immutable for the deployed contract; any change requires a new deployment and a fresh review.

## 6. Router batch limits

- [ ] The documented maximum invocation count and maximum argument size enforced by `Router.exec` are recorded in the release notes.
- [ ] The router limits were reviewed out-of-band by a second person (mainnet: **required**) and match the values documented in the contract.
- [ ] Boundary behavior is confirmed: oversized batches are rejected with the typed error before dispatch, and accepted batches execute atomically.
- [ ] Any change to the router limits is called out in the release notes as a breaking change for callers.

## 7. Rollback and incident readiness

- [ ] A rollback plan is documented: previous known-good commit SHA and WASM checksum are recorded.
- [ ] Rollback has been rehearsed or validated on testnet.
- [ ] Incident contacts and escalation path are recorded in the release notes.
- [ ] Monitoring and alerting are active for the deployed contract before traffic is enabled.
- [ ] A post-deployment verification step (smoke test) is defined and assigned.
- [ ] The recorded `CLAIM_DEADLINE` and router batch limits are included in the rollback plan so a rollback restores the same schedule and limits.

## Sign-off

| Role | Name | Date | Commit SHA | WASM SHA-256 |
| --- | --- | --- | --- | --- |
| Release executor | | | | |
| Reviewer (mainnet required) | | | | |

Record sign-off in the release notes or issue tracker — **not** in this file with any sensitive values.