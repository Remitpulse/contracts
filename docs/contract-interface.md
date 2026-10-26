# Contract Interface & Event Reference

This document is the maintained reference for every public entrypoint exposed by the
contracts in this repository. It records, for each entrypoint:

- **Authorization** — who may call it and what must be proven.
- **Input validation** — constraints the contract enforces on arguments.
- **Storage effects** — persistent state that is read or written.
- **Events** — events emitted (topic / data shape).
- **Errors** — error codes or messages that may be returned.

> **Maintenance requirement:** this document MUST be updated in the same pull request
> whenever a public entrypoint, its authorization, its storage effects, its events, or
> its error surface changes. Reviewers should treat an out-of-date interface reference
> as a blocking issue. See the "Pull-request expectations" section at the end.

Contracts covered:

- [`account`](#account-contract) — session and passkey management.
- [`airdrop`](#airdrop-contract) — claim and recover flows.
- [`nft`](#nft-contract) — mint, transfer, burn.
- [`router`](#router-contract) — `exec`.
- [`subsidy`](#subsidy-contract) — `reimburse`.
- [`webauthn`](#webauthn-contract) — `verify`.

Conventions used below:

- **Caller** refers to the transaction signer / invoker identity as seen by the contract.
- **Admin** refers to the privileged address configured at initialization.
- **Auth** values: `none` (permissionless), `caller` (must be the authenticated caller),
  `admin` (must be the configured admin), `signature` (must present a valid signature),
  `passkey` (must present a valid WebAuthn assertion).
- All amounts are in the contract's smallest unit unless stated otherwise.

---

## `account` contract

Manages account sessions and passkey credentials. The account contract is the
authorization root for user actions: sessions and passkeys are the two mechanisms by
which a user authorizes calls.

### constructor

- **Auth:** `none` (invoked once at deployment).
- **Input validation:**
  - Must not already be initialized; admin address must be non-zero.
  - Each initial passkey public key must be an uncompressed P-256 (SEC1) point:
    a valid `0x04`-prefixed encoding of a point on the P-256 curve.
  - The RP ID binding must be a 32-byte hash (e.g. SHA-256 of the RP ID).
  - The origin must be a canonical HTTPS origin (scheme `https`, a host, no path,
    query, fragment, or trailing slash).
- **Storage effects:** writes admin address, initial passkeys, RP ID hash, origin,
  and initialization flag.
- **Events:** none.
- **Errors:** `AlreadyInitialized`, `InvalidAdmin`, `InvalidPasskey`, `InvalidRpIdHash`,
  `InvalidOrigin`.

### `create_session`

Creates a scoped session key that can act on behalf of the account until expiry.

- **Auth:** `caller` (the account owner) or a valid `passkey` assertion.
- **Input validation:**
  - Session public key must be non-zero and not already registered.
  - `expires_at` must be in the future.
  - Scope/permissions must be a subset of the owner's permissions.
- **Storage effects:** writes a new session record (key, scope, `expires_at`).
- **Events:** `session_created` — topics: `[session_created, account]`; data: `(session_key, expires_at, scope)`.
- **Errors:** `Unauthorized`, `SessionExists`, `InvalidExpiry`, `ScopeExceedsOwner`.

### `revoke_session`

Revokes an existing session key.

- **Auth:** `caller` (owner) or `admin`.
- **Input validation:** session key must exist.
- **Storage effects:** removes/marks the session record as revoked.
- **Events:** `session_revoked` — topics: `[session_revoked, account]`; data: `(session_key)`.
- **Errors:** `Unauthorized`, `SessionNotFound`.

### `register_passkey`

Registers a WebAuthn credential for the account.

- **Auth:** `caller` (owner) or `admin`.
- **Input validation:**
  - Credential ID must be non-empty and not already registered.
  - Public key must be a valid encoded COSE key.
  - Signature counter must be initialized to `0`.
- **Storage effects:** writes a passkey record (credential ID, public key, counter).
- **Events:** `passkey_registered` — topics: `[passkey_registered, account]`; data: `(credential_id)`.
- **Errors:** `Unauthorized`, `PasskeyExists`, `InvalidCredential`.

### `remove_passkey`

Removes a registered passkey credential.

- **Auth:** `caller` (owner) or `admin`.
- **Input validation:** credential ID must exist.
- **Storage effects:** deletes the passkey record.
- **Events:** `passkey_removed` — topics: `[passkey_removed, account]`; data: `(credential_id)`.
- **Errors:** `Unauthorized`, `PasskeyNotFound`.

### `authenticate_passkey`

Verifies a WebAuthn assertion and, on success, authorizes the requested action.

- **Auth:** `passkey` (assertion over the challenge and action payload).
- **Input validation:**
  - Credential ID must be registered.
  - Assertion signature must verify against the stored public key.
  - Signature counter must be strictly greater than the stored counter (replay protection).
  - Challenge must match the expected value for the action.
- **Storage effects:** updates the stored signature counter.
- **Events:** `passkey_authenticated` — topics: `[passkey_authenticated, account]`; data: `(credential_id, new_counter)`.
- **Errors:** `PasskeyNotFound`, `InvalidSignature`, `CounterReplay`, `ChallengeMismatch`.

### `is_authorized`

Read-only authorization check used by other contracts.

- **Auth:** `none`.
- **Input validation:** account and action must be provided.
- **Storage effects:** reads session/passkey records only.
- **Events:** none.
- **Errors:** none (returns boolean).

---

## `airdrop` contract

Distributes tokens to eligible recipients with a claim and a recover path.

### `init`

- **Auth:** `none` (deployment only).
- **Input validation:** must not already be initialized; admin non-zero; token address non-zero.
- **Storage effects:** writes admin, token address, initialization flag.
- **Events:** none.
- **Errors:** `AlreadyInitialized`, `InvalidAdmin`, `InvalidToken`.

### `set_merkle_root`

Sets the eligibility Merkle root for a distribution round.

- **Auth:** `admin`.
- **Input validation:** root must be non-zero; round must not be finalized.
- **Storage effects:** writes the Merkle root for the round.
- **Events:** `root_updated` — topics: `[root_updated, round]`; data: `(root)`.
- **Errors:** `Unauthorized`, `RoundFinalized`, `InvalidRoot`.

### `claim`

Claims an allocation for the caller using a Merkle proof.

- **Auth:** `caller` (recipient).
- **Input validation:**
  - `(caller, amount)` must be a leaf in the tree for the current root.
  - Proof must verify against the stored root.
  - Allocation must not already have been claimed.
  - `amount` must be greater than zero.
- **Storage effects:** marks the allocation claimed; transfers tokens to the caller.
- **Events:** `claimed` — topics: `[claimed, caller]`; data: `(amount, round)`.
- **Errors:** `Unauthorized`, `InvalidProof`, `AlreadyClaimed`, `InvalidAmount`, `TransferFailed`.

### `recover`

Recovers unclaimed or misdirected funds back to the admin/treasury after the claim
window closes.

- **Auth:** `admin`.
- **Input validation:**
  - Claim window must have expired.
  - Recoverable amount must be greater than zero.
- **Storage effects:** decrements the round's remaining balance; transfers tokens to the admin.
- **Events:** `recovered` — topics: `[recovered, round]`; data: `(amount)`.
- **Errors:** `Unauthorized`, `WindowNotExpired`, `NothingToRecover`, `TransferFailed`.

### `is_claimed`

Read-only claim status.

- **Auth:** `none`.
- **Input validation:** recipient address required.
- **Storage effects:** reads claim records only.
- **Events:** none.
- **Errors:** none (returns boolean).

---

## `nft` contract

Non-fungible token with mint, transfer, and burn.

### `init`

- **Auth:** `none` (deployment only).
- **Input validation:** must not already be initialized; admin non-zero; name/symbol non-empty.
- **Storage effects:** writes admin, name, symbol, initialization flag.
- **Events:** none.
- **Errors:** `AlreadyInitialized`, `InvalidAdmin`, `InvalidMetadata`.

### `mint`

Mints a new token to a recipient.

- **Auth:** `admin`.
- **Input validation:** recipient non-zero; token ID must not already exist.
- **Storage effects:** writes token ownership record; increments total supply; assigns token ID.
- **Events:** `mint` — topics: `[mint, recipient, token_id]`; data: `(token_id)`.
- **Errors:** `Unauthorized`, `InvalidRecipient`, `TokenExists`.

### `transfer`

Transfers a token from the caller to a recipient.

- **Auth:** `caller` (current owner) or an approved operator.
- **Input validation:** token must exist; caller must be owner or approved; recipient non-zero and not the current owner.
- **Storage effects:** updates token ownership; clears prior approval.
- **Events:** `transfer` — topics: `[transfer, from, to, token_id]`; data: `(token_id)`.
- **Errors:** `Unauthorized`, `TokenNotFound`, `InvalidRecipient`, `NotApproved`.

### `approve`

Grants an operator permission to transfer a specific token.

- **Auth:** `caller` (current owner).
- **Input validation:** token must exist; caller must be owner; operator non-zero.
- **Storage effects:** writes per-token approval.
- **Events:** `approval` — topics: `[approval, owner, operator, token_id]`; data: `(token_id)`.
- **Errors:** `Unauthorized`, `TokenNotFound`, `InvalidOperator`.

### `burn`

Burns a token.

- **Auth:** `caller` (current owner) or `admin`.
- **Input validation:** token must exist; caller must be owner or admin.
- **Storage effects:** deletes token ownership record; decrements total supply.
- **Events:** `burn` — topics: `[burn, owner, token_id]`; data: `(token_id)`.
- **Errors:** `Unauthorized`, `TokenNotFound`.

### `owner_of`

Read-only ownership lookup.

- **Auth:** `none`.
- **Input validation:** token must exist.
- **Storage effects:** reads ownership record only.
- **Events:** none.
- **Errors:** `TokenNotFound`.

### `bulk_mint_with_data`

Mints multiple tokens with associated data in a single call.

- **Auth:** `admin`.
- **Input validation:**
  - The batch must not exceed `MAX_BULK_MINT` (a conservative maximum batch size).
  - Remaining supply must be at least the batch length; capacity is validated
    **before any mutation**, so an over-capacity batch fails without minting.
  - Recipients must be non-zero; token IDs must not already exist.
- **Storage effects:** writes token ownership records and associated data; increments
  total supply.
- **Events:** `mint` — topics: `[mint, recipient, token_id]`; data: `(token_id)`.
- **Errors:** `Unauthorized`, `MaxBatchExceeded`, `InsufficientSupply`,
  `InvalidRecipient`, `TokenExists`.

### `get_owner_tokens`

Returns the complete list of token IDs owned by an address.

- **Auth:** `none`.
- **Input validation:** owner address required.
- **Storage effects:** reads the owner's token list.
- **Events:** none.
- **Errors:** none.
- **Deprecated:** retained for compatibility; prefer `get_owner_tokens_page`, whose
  cost is bounded. This endpoint's read, serialization, and response cost grow without
  bound as collections grow.

### `get_owner_tokens_page`

Returns a bounded page of token IDs owned by an address.

- **Auth:** `none`.
- **Input validation:**
  - `limit` must be greater than zero and at most `MAX_OWNER_TOKENS_PAGE`.
  - `cursor` must be `None` (first page) or a token ID previously returned as the
    page's next cursor.
- **Ordering:** results are returned in stable ascending token-ID order; pagination
  preserves this ordering across calls.
- **Return shape:** the page of token IDs plus the next cursor (`None` when the page
  is the final page).
- **Storage effects:** reads the owner's token list; extends the persistent TTL of
  the owner-token storage entry.
- **Events:** none.
- **Errors:** `InvalidCursor`.

---

## `router` contract

Executes batched calls to target contracts.

### `exec`

Executes one or more calls atomically.

- **Auth:** `caller` (the invoker authorizes each sub-call via its own mechanism).
- **Input validation:**
  - Each call must specify a valid target and non-empty calldata.
  - Total calls must not exceed the configured batch limit.
  - Target must be on the allowlist (if an allowlist is configured).
- **Storage effects:** delegates to targets; may update router bookkeeping (nonces/counters) if configured.
- **Events:** `executed` — topics: `[executed, caller]`; data: `(call_count, success)`.
- **Errors:** `Unauthorized`, `InvalidTarget`, `EmptyCalldata`, `BatchTooLarge`, `TargetNotAllowed`, `CallFailed`.

### `set_allowlist`

Configures the target allowlist.

- **Auth:** `admin`.
- **Input validation:** entries must be non-zero addresses.
- **Storage effects:** writes the allowlist.
- **Events:** `allowlist_updated` — topics: `[allowlist_updated]`; data: `(entries)`.
- **Errors:** `Unauthorized`, `InvalidTarget`.

---

## `subsidy` contract

Reimburses eligible callers for subsidized operations.

### `init`

- **Auth:** `none` (deployment only).
- **Input validation:** must not already be initialized; admin non-zero; token non-zero.
- **Storage effects:** writes admin, token, initialization flag.
- **Events:** none.
- **Errors:** `AlreadyInitialized`, `InvalidAdmin`, `InvalidToken`.

### `reimburse`

Reimburses a caller for a qualifying operation.

- **Auth:** `caller` (the subsidized party) or `admin` on behalf of a party.
- **Input validation:**
  - Requested amount must be greater than zero and within the per-call cap.
  - Caller must not exceed the per-period quota.
  - The referenced operation must be eligible (if eligibility is enforced).
- **Storage effects:** increments the caller's usage/quota counters; transfers tokens to the caller.
- **Events:** `reimbursed` — topics: `[reimbursed, caller]`; data: `(amount, period)`.
- **Errors:** `Unauthorized`, `InvalidAmount`, `ExceedsCap`, `QuotaExceeded`, `NotEligible`, `TransferFailed`.

### `set_cap`

Sets the per-call cap and per-period quota.

- **Auth:** `admin`.
- **Input validation:** cap and quota must be non-negative; cap must not exceed the treasury balance policy.
- **Storage effects:** writes cap and quota configuration.
- **Events:** `cap_updated` — topics: `[cap_updated]`; data: `(cap, quota)`.
- **Errors:** `Unauthorized`, `InvalidAmount`.

---

## `webauthn` contract

Verifies WebAuthn assertions on-chain for use by the account contract.

### `verify`

Verifies a WebAuthn assertion against a supplied public key and challenge.

- **Auth:** `none` (pure verification; callers enforce the result).
- **Input validation:**
  - Credential public key must be a valid encoded COSE key.
  - Challenge must be non-empty and match the expected value.
  - Authenticator data must be well-formed and include the required flags (e.g. user-present).
  - Client data JSON hash must match the signed payload.
  - Signature must verify against the public key over the signed payload.
- **Storage effects:** none (stateless verification).
- **Events:** none.
- **Errors:** `InvalidCredential`, `ChallengeMismatch`, `MalformedAuthenticatorData`, `InvalidSignature`.

---

## Pull-request expectations

Documentation is part of the change, not an afterthought. For any pull request that
touches a contract's public surface:

1. **Update this file** if you add, remove, or change a public entrypoint, its
   authorization, its input validation, its storage effects, its emitted events, or
   its error surface.
2. **Keep it derived from source.** When in doubt, the contract source is the source of
   truth; this document must match it. If a discrepancy is found, fix the document (or
   the code) in the same PR.
3. **Reviewers must check** that the interface reference reflects the change. A PR that
   changes a public entrypoint without updating this document should be blocked.
4. **Events and errors** listed here must correspond to the identifiers actually emitted
   or returned by the contracts.

_Last reviewed against source: see the git history of this file._