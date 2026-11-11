# Contract interfaces

This reference describes the source in this workspace. It is reviewed manually;
CI does not currently compare documentation with generated contract specs.
All token amounts are signed i128 base units. See the source enums for XDR types.

## Account

`__constructor(Config { keys: Vec<BytesN<65>>, threshold: u32,
rp_id_hash: BytesN<32>, origin: Bytes })` stores immutable configuration.
There must be 1–10 unique keys and a threshold of 1 through key count.
Keys require SEC1 prefix 04 and coordinates below the P-256 field prime;
**the constructor does not establish curve membership or key possession**.
Deployment preflight checks curve membership. RP hash must be nonzero.
Origin is at most 256 bytes, begins `https://`, and rejects whitespace,
queries, fragments and trailing slash; enrollment must enforce canonical origin
and RP-ID relationships. No administrator, factory, recovery or key rotation exists.

`Signature` is the tuple enum `Passkeys(Vec<Assertion>)` or
`Session(BytesN<32>, BytesN<64>)`. An `Assertion` is a symbol-keyed map containing
`key_index: u32`, `authenticator_data: Bytes`, `client_data_json: Bytes`, and
`signature: BytesN<64>` (raw low-S r||s). Tuple enums encode as a vector beginning
with the variant symbol. Credential IDs are not stored; enrollment maps each
credential to a configured key index.

| Entrypoint | Authorization and effect |
| --- | --- |
| `add_session(public_key: BytesN<32>, session: Session)` | Account auth through full passkey quorum. Replaces the record/budget. Requires positive `per_transfer`, `remaining >= per_transfer`, future timestamp expiry. |
| `revoke_session(public_key)` | Full quorum; removes record, including when absent. |
| `set_sessions_paused(paused: bool)` | Full quorum; freezes/resumes session authorization only. |
| `config()` | Public immutable configuration getter. |
| `session(public_key)` | Public optional session getter. |
| `sessions_paused()` | Public bool getter, false initially. |
| `__check_auth(payload, signature, contexts)` | Host custom-account callback; validates quorum or bounded session, then renews instance TTL. It is not a general callable authorization oracle. |

`Session` fields: `token: Address`, `recipient: Option<Address>`,
`per_transfer: i128`, `remaining: i128`, `expires_at: u64` (Unix seconds).
Session contexts must all be the selected token's `transfer(account,to,amount)`;
positive individual amounts must satisfy the cap and checked aggregate budget.
Expiry rejects at equality. Any configured recipient must match. Ed25519 verifies
the host authorization payload. Budget updates roll back on transaction failure.
Events: topics `(session,set,key)` / Session, `(session,revoked,key)` / (),
`(session,paused)` / bool, `(session,spent,key)` / (spent,remaining).
Errors: Invalid=1, Expired=2, Scope=3, Limit=4, Paused=5,
InvalidThreshold=6, InvalidKeyCount=7, InvalidPublicKey=8,
InvalidRpIdHash=9, InvalidOrigin=10. Cryptographic/validation failures may trap.

## WebAuthn

`verify(key, rp_id_hash, origin, payload, assertion)` is a public stateless verifier
with caller-supplied trust inputs. It grants no authority by itself. The account
links the same verifier library and supplies its stored trust inputs.
Verification checks RP hash, UP+UV flags, exact `webauthn.get` type, origin,
non-cross-origin ceremony, unpadded base64url host payload challenge and ES256.
Client JSON is limited to 2048 bytes and must parse completely. Compared strings
must be literal, not escaped JSON strings. Unknown fields are allowed; duplicate
known fields are rejected by serde. No credential enrollment or sign counter
tracking occurs here. No state/events. Failures trap.

## Subsidy

`__constructor(admin: Address)` stores immutable administrator and epoch zero.
`admin()`, `active_epoch()`, `is_reimbursed(id: BytesN<32>)` are public getters.
`reimburse(id, asset: Address, recipient: Address, amount: i128)` requires admin
auth, positive amount, unused active-epoch ID, available epoch capacity and enough
vault token balance. It marks the ID before transfer and emits topics
`(subsidy,id)` with `(asset,recipient,amount)`. All writes roll back on failure.
`rotate_epoch()` requires admin auth, removes active markers and the ID list,
checks u32 increment and emits `(epoch)` / next epoch. Old IDs become reusable.
Each epoch accepts at most 128 reimbursements to bound instance storage.
Errors: InvalidAmount=1, AlreadyReimbursed=2, InsufficientBalance=3,
EpochFull=4, EpochExhausted=5. Token/auth host errors may trap.
Storage layout changed to add an epoch ID index; this is intended for new
instances, not an in-place migration of previously deployed vaults.

## Router

`exec(caller: Address, invocations: Vec<(Address,Symbol,Vec<Val>)>) -> Vec<Val>`
requires caller auth. Up to 10 invocations, each with argument vector XDR at most
1024 bytes, are validated before dispatch. Targets enforce their own auth; the
router does not grant caller authority over another address. Failures revert the
whole transaction. No router events. Errors: TooManyInvocations=1,
ArgumentTooLarge=2. Instance TTL is renewed. Result sizes and arbitrary target
work remain bounded by network transaction limits, not by these input caps.

## Airdrop

`__constructor(root_hash: BytesN<32>, token: Address, admin: Address,
funder: Address, claim_deadline: u32)` requires admin auth and a future **ledger
sequence**, stores immutable distribution configuration, and sets the OZ Merkle
root (event `set_root`).
`claim(index: u32, receiver: Address, amount: i128, proof: Vec<BytesN<32>>)`
requires receiver auth, an open claim window and an unclaimed valid Merkle leaf
binding index, receiver and amount. OZ records the persistent claim marker and
emits `set_claimed`; SEP-41 transfer occurs atomically. Token contracts reject
invalid amounts; publishers must generate positive, uniquely indexed leaves.
`recover_unclaimed()` requires admin auth and sequence >= deadline, marks ended,
transfers any positive balance to the fixed funder, emits `(recovered,admin)` /
`(funder,remaining)` and renews instance TTL. Zero balance recovery is allowed.
`claim_deadline()`, `is_ended()`, `is_claimed(index)` are public getters.
Local errors: Ended=1000, ClaimWindowOpen=1001, InvalidClaimDeadline=1002,
ClaimWindowClosed=1003; OZ proof/claimed errors and token/auth traps also apply.
Persistent claim records must be restored if archived, never treated as fresh.

## NFT

`__constructor(owner: Address,max_supply: u32,metadata: TokenMetadata)` fixes
owner and lifetime mint cap; metadata has `base_uri`, `name`, `symbol` strings.
`set_metadata_uri(base_uri)`, `mint(to,token_id)`,
`mint_with_data(to,token_id,data)` and
`bulk_mint_with_data(Vec<(Address,u32,TokenData)>)` require owner auth.
TokenData contains `session_id` and `resource` strings. Bulk mint is capped at 50.
`total_supply()` counts lifetime mints (does not decrement on burn).
`get_max_supply()`, `get_token_data(token_id)`, `get_token_metadata()`,
`get_owner_tokens(owner)`, `get_owner_tokens_page(owner,cursor,limit)` are public.
Pagination requires limit 1–100 and an existing token-ID cursor, sorted ascending.
Owner lists still load the entire collection; pagination does not bound storage I/O.

`balance(owner)`, `owner_of(token_id)`, `transfer(from,to,token_id)`,
`transfer_from(spender,from,to,token_id)`,
`approve(approver,approved,token_id,live_until_ledger)`,
`approve_for_all(owner,operator,live_until_ledger)`, `get_approved(token_id)`,
`is_approved_for_all(owner,operator)`, `burn(from,token_id)` and
`burn_from(spender,from,token_id)` delegate authorization, approval expiry and
standard events to OpenZeppelin v0.3.0. Transfers/burns also update local owner
lists. Approval-for-all lives in temporary storage; token approvals are persistent
with a logical expiry checked by OZ. Local code must not apply persistent TTL
operations to temporary approval keys. Local error codes are in
`contracts/nft/src/errors.rs`; upstream errors are in the pinned dependency.
