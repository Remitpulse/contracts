# Security model

WebAuthn assertions are bound to the Soroban authorization hash, configured RP
ID hash, and exact origin. The verifier requires user presence and user
verification. Soroban supplies the network-bound authorization payload, nonce,
and expiration-ledger replay controls.

Session keys are Ed25519 keys constrained by expiry, token, optional recipient,
per-transfer cap, and remaining budget. They may authorize only SEP-41 transfers
from the account. A passkey quorum can revoke a session or pause all sessions.

The subsidy vault requires administrator authorization, positive amounts, unique
IDs, and sufficient balance. It does not sponsor Stellar fees or enforce an
off-chain fee policy. The generic router deliberately has no allowlist and must
not be treated as a policy boundary.

No recovery, key rotation, or upgrade path is implemented. A safe design would
need a recovery quorum, delay, user-visible confirmation, cancellation, and an
emergency freeze; it remains a future improvement pending threat modeling.
