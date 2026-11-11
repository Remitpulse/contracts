# Consumer compatibility review — 2026-11-05 follow-up

This is a source-level interface comparison, not an end-to-end integration test
or production-readiness claim. No frontend/backend changes or network transactions
were made. Branch heads were re-read through GitHub, not inferred from old clones.

| Repository | Reviewed revision |
| --- | --- |
| `Remitpulse/contracts` main | `f1b9b52212f4cba2c2c2735ff29cef8713f07c66` (still the broken baseline described in PR #38) |
| Contract fixes in PR #38, `fix/contracts-readiness` | `433ac055553eb8324ad7ee4143c3f0e2624a0f4f`; source tree `976063522c66031cfd5de1d9aed8bb1391d7504a` |
| `Remitpulse/frontend` main | `7319d4e3cc4a27a477c8718eb34fd865cfff8339` |
| `Remitpulse/backend` main | `b617e2fa08d3b0020d0d1dd967c0263a03c121ba` |

This follow-up starts at the published #38 head and changes no contract entrypoint,
storage layout, authorization policy or signature type. The contract column below
refers to that fixed implementation. A successful component unit suite does not
establish compatibility among these three repositories.

Sources: frontend [signatures](https://github.com/Remitpulse/frontend/blob/7319d4e3cc4a27a477c8718eb34fd865cfff8339/lib/stellar/signatures.ts),
[smart-account transaction builder](https://github.com/Remitpulse/frontend/blob/7319d4e3cc4a27a477c8718eb34fd865cfff8339/lib/stellar/smartAccount.ts),
[session-key flow](https://github.com/Remitpulse/frontend/blob/7319d4e3cc4a27a477c8718eb34fd865cfff8339/hooks/useSessionKey.ts),
[HTTP client](https://github.com/Remitpulse/frontend/blob/7319d4e3cc4a27a477c8718eb34fd865cfff8339/lib/api/client.ts);
backend [contract bindings](https://github.com/Remitpulse/backend/blob/b617e2fa08d3b0020d0d1dd967c0263a03c121ba/src/interfaces/contracts/bindings.ts),
[session service](https://github.com/Remitpulse/backend/blob/b617e2fa08d3b0020d0d1dd967c0263a03c121ba/src/api/session-keys/service.ts),
[wallet submission](https://github.com/Remitpulse/backend/blob/b617e2fa08d3b0020d0d1dd967c0263a03c121ba/src/api/wallet/invocation-service.ts),
[wallet routes](https://github.com/Remitpulse/backend/blob/b617e2fa08d3b0020d0d1dd967c0263a03c121ba/src/api/wallet/routes.ts).
Contract schemas are in [the interface reference](contract-interface.md) and source.

## Contract ABI and authorization

| Area | Fixed contract | Frontend main | Backend main | Assessment / required change |
| --- | --- | --- | --- | --- |
| Wallet creation | Deploy account WASM with `Config {keys,threshold,rp_id_hash,origin}`; no factory | Registration expects an already deployed `contractId` | `AccountFactory.deploy(salt,key_id,public_key)` / `get_address(salt)` | **Blocked.** Design a real factory or constructor-based deployment flow. A renamed function cannot supply missing RP/origin/quorum policy. |
| Passkey signature XDR | Tuple enum `Passkeys(Vec<Assertion>)`; each assertion includes `key_index` | `WebAuthn` variant with map including credential `id` | Bare map with auth/client JSON/signature | **Blocked.** Both encodings fail the contract schema. Add the correct enum/vector, index mapping and quorum collection. |
| Ed25519 signature XDR | Tuple enum `Session(public_key,signature)` (three vector elements) | `Ed25519` variant with one nested map | Bare `public_key`/`signature` map | **Blocked.** Encode the exact tuple enum. Signature length compatibility alone is insufficient. |
| Credential trust | Immutable unique SEC1 keys, quorum threshold, exact RP hash and origin | Stores one credential ID/session; prompts one signer | Verifies one DB credential per submit | **Blocked for quorum >1.** Map credential IDs to immutable on-chain key indices; collect distinct assertions over the same payload. Verify possession before funding. |
| Passkey additions/recovery | No key rotation, recovery or upgrade endpoint | Credential/session model does not synchronize contract key configuration | `/v1/auth/passkey/add/*` adds credentials in the database | **Blocked as on-chain rotation.** DB enrollment does not authorize a new on-chain key; agree a migration/recovery policy. |
| WebAuthn signed message | Unpadded base64url of Soroban's 32-byte auth payload; UP+UV, `webauthn.get`, exact origin, no crossOrigin | Uses auth payload as challenge and requests UV | `contractSigningOptions`/`verifyContractAssertion` use auth payload and require UV | **Aligned in intent**, but wrong encodings above still prevent successful authorization. Runtime RP/origin configuration is unverified. |
| ES256 signature form | Raw 64-byte low-S r||s | DER conversion and low-S normalization | DER-to-compact conversion with low-S normalization | **Aligned in source**, subject to full signed assertion integration tests. |
| Replay controls | Host network ID, nonce and expiration ledger bound to signed invocation; no WebAuthn sign counter storage | `authorizeEntry` takes network passphrase and ledger TTL | Computes host authorization preimage and attaches expiration/signature | **Aligned mechanism**, not a successful integration claim. Backend DB counters are additional policy, not contract replay storage. |
| Session creation | `add_session(key, Session)`; Session includes token, optional recipient, per_transfer, remaining, expires_at | Requests `/session-keys`, expects unsigned `add_signer` XDR | Calls `add_session_key(key,token,perTx,total,expiresAtLedger)` | **Blocked.** Name, arity, struct encoding and scope fields differ. |
| Session expiry | u64 absolute ledger timestamp in Unix seconds, rejects at equality | Local `Date.now()` milliseconds and TTL minutes | Derives `expiresAtLedger` u32 for on-chain call, plus separate DB Date | **Blocked.** Distinguish session timestamp from auth-entry expiration ledger and client milliseconds. |
| Session scope/budget | Exact token, optional fixed recipient, positive i128 per-transfer and aggregate remaining budget | One active local record, floating maxAmount; no token or aggregate budget in issue request | Token/per-tx/total budget in DB and prepare call; no recipient binding input | **Blocked / missing policy.** Expose all intended contract restrictions and use exact base units. Client limits cannot replace on-chain registration. |
| Revocation/pause | `revoke_session(key)` and quorum `set_sessions_paused(bool)` | Clearing local IndexedDB stops that browser signing; no equivalent contract pause integration | Revoke prepares `revoke_session_key`; immediately disables backend relay | **Blocked.** Local/DB deletion is not on-chain revocation; encode real entrypoints and expose incident freeze. |
| Generic invocation endpoint | Session/Config are maps; signatures are tagged vectors | Expects full prepared transaction XDR | TypedArg accepts scalar address/int/bool/symbol/string/bytes only | **Blocked for structured calls.** Renaming the session method alone is insufficient; add reviewed struct/collection encoding. |
| Token transfer | SEP-41 `transfer(from,to,amount)` | Same three arguments, i128 base units | Same three arguments, i128 base units | **ABI aligned**, but all auth/signature/relay blockers still apply. |
| Token precision | Asset-specific native base units; no assumed decimal count | Defaults to seven decimals and truncates extra fractional digits in `toBaseUnits` | Fixed seven decimals; rejects excess precision | **Conditionally aligned only for verified seven-decimal assets.** Query/verify decimals and agree reject-vs-round policy; never infer identity from ticker. |
| Subsidy vault | Admin-only `reimburse(id,asset,recipient,amount)`; bounded active epochs, IDs reusable after rotation | Configures subsidy ID; no matching reimbursement admin flow identified | Calls `allowance(account)` and `claim(account)` | **Blocked.** Those endpoints do not exist. Choose off-chain reimbursement/fee policy instead of a permissive compatibility alias. |
| Fee sponsorship/relay | Vault pays tokens, not transaction fees | Builds auth-bearing envelope from random simulation source; expects relay rebuilding before fee bump | Prepared invocation flow rebuilds with relayer; direct paymaster route requires an already signed inner source | **Blocked as a combined flow.** Adopt one versioned relay protocol and preserve signed invocation/nonce/expiry; a fee bump cannot repair an unsigned inner source. |
| Router/airdrop/NFT | Generic router; immutable indexed Merkle distribution; owner-administered NFT | No matching direct integrations identified in reviewed signing/API paths | No dedicated typed adapters in contract bindings | **Not integrated.** Do not infer coverage from generic invocation support. |

## Frontend-to-backend HTTP protocol

A configurable base URL can supply `/v1`; it cannot reconcile the method suffixes,
required bodies and response schemas below.

| Flow | Frontend main | Backend main | Assessment |
| --- | --- | --- | --- |
| Registration | `/auth/register/options` with displayName; verify includes publicKeyHex | `/v1/auth/passkey/register/*` requires email, verifies credential server-side | Incompatible routes and enrollment authority. |
| Login/session | `/auth/login/*`, options without email; direct `{token,contractId,credentialId,displayName}` | `/v1/auth/passkey/login/*` requires email; `{data:{token,user}}`; wallet deployed separately | Incompatible discovery and account/session model. |
| Response/errors | Reads successful JSON directly and top-level message/code on failure | ApiRouter wraps success in `{data:...}`; errors use `{error:{code,message,details}}` | No shared response envelope. |
| Session issue/confirm | Raw publicKeyHex, contractId, maxAmount number, ttlMinutes; expects `{xdr}` then `/session-keys/confirm` with signedXdr | `/v1/wallet/session-keys`: G... key, tokenContract, perTxLimit, totalLimit, ttlSeconds; returns session + invocation; submit by invocation ID with assertion/signature | Incompatible key representation, units, scope and submission API. |
| Payments | `/transfers` includes signedXdr/quote data; transfer status and SSE | `/v1/wallet/payments` prepares an invocation; `/invocations/:id/submit` accepts assertion/signature; no matching transfer/SSE protocol | No executable shared payment flow. |
| Quotes/recipient/balances | `/quotes`, `/recipients`, `/accounts/:id/balance` | FX/corridor/wallet routes, without equivalent recipient/list/quote settlement contract | Product-level integration needed; no safe alias implying settlement. |
| SEP-24 | `/sep24/interactive` and `/sep24/transactions/:id` | Anchor deposit/withdraw and remittance routes | Paths, ownership and response states differ. |

## Required acceptance evidence

Before integrating funded wallets, maintainers must agree the factory/enrollment,
quorum and subsidy policy; implement a versioned ABI/HTTP contract; and run genuine
cross-repository tests against reviewed WASM. Include two distinct quorum signers,
timestamp/ledger boundary cases, wrong token/recipient, aggregate spending, failed
transfer rollback, revocation/pause, auth nonce reuse, restored state and real
relayer source signing. Demo success and isolated mocked HTTP tests do not replace
that evidence. This follow-up does not add permissive contract aliases or change
authorization boundaries to make incompatible clients appear functional.
