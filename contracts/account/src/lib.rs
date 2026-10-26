//! RemitPulse passkey multisig account with scoped, expiring session keys.
#![no_std]
use remitpulse_webauthn_verifier::{verify_assertion, Assertion};
use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env,
    Symbol, TryFromVal, Vec,
};

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub keys: Vec<BytesN<65>>,
    pub threshold: u32,
    pub rp_id_hash: BytesN<32>,
    pub origin: Bytes,
}
#[contracttype]
#[derive(Clone)]
pub struct Session {
    /// Network-specific SEP-41/SAC token address: USDC, EURC, ARST, BRLA, etc.
    pub token: Address,
    /// Some(address) restricts this key to a single remittance beneficiary.
    pub recipient: Option<Address>,
    /// Amounts are token base units, never a cross-currency USD valuation.
    pub per_transfer: i128,
    pub remaining: i128,
    pub expires_at: u64,
}
#[contracttype]
#[derive(Clone)]
pub enum Signature {
    Passkeys(Vec<Assertion>),
    Session(BytesN<32>, BytesN<64>),
}
#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    SessionsPaused,
    Session(BytesN<32>),
}
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Invalid = 1,
    Expired = 2,
    Scope = 3,
    Limit = 4,
    Paused = 5,
    InvalidThreshold = 6,
    InvalidKeyCount = 7,
    InvalidPublicKey = 8,
    InvalidRpIdHash = 9,
    InvalidOrigin = 10,
}
#[contract]
pub struct Account;

fn touch(e: &Env) {
    e.storage().instance().extend_ttl(120_960, 518_400);
}

/// P-256 field prime p = 0xFFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF.
const P256_P: [u8; 32] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// Returns true if the 32-byte big-endian coordinate is strictly less than the P-256 field prime.
fn coord_in_range(coord: &BytesN<32>) -> bool {
    let mut bytes = [0u8; 32];
    coord.copy_into_slice(&mut bytes);
    for i in 0..32 {
        if bytes[i] < P256_P[i] {
            return true;
        }
        if bytes[i] > P256_P[i] {
            return false;
        }
    }
    // Equal to p is out of range.
    false
}

/// Validates an uncompressed SEC1 P-256 point: 65 bytes, 0x04 prefix, x and y each < p.
fn validate_public_key(e: &Env, key: &BytesN<65>) {
    if key.get(0) != Some(4) {
        e.panic_with_error(Error::InvalidPublicKey);
    }
    let mut x = [0u8; 32];
    let mut y = [0u8; 32];
    for i in 0..32 {
        x[i] = key.get(1 + i as u32).unwrap();
        y[i] = key.get(33 + i as u32).unwrap();
    }
    let x = BytesN::from_array(e, &x);
    let y = BytesN::from_array(e, &y);
    if !coord_in_range(&x) || !coord_in_range(&y) {
        e.panic_with_error(Error::InvalidPublicKey);
    }
}

/// Validates the RP ID hash is a non-zero 32-byte value.
fn validate_rp_id_hash(e: &Env, rp_id_hash: &BytesN<32>) {
    let mut bytes = [0u8; 32];
    rp_id_hash.copy_into_slice(&mut bytes);
    if bytes.iter().all(|b| *b == 0) {
        e.panic_with_error(Error::InvalidRpIdHash);
    }
}

/// Validates a canonical HTTPS origin: "https://" scheme, non-empty host, no whitespace,
/// no trailing slash, and no query or fragment.
fn validate_origin(e: &Env, origin: &Bytes) {
    let len = origin.len();
    if len == 0 {
        e.panic_with_error(Error::InvalidOrigin);
    }
    let mut buf = [0u8; 256];
    if len as usize > buf.len() {
        e.panic_with_error(Error::InvalidOrigin);
    }
    origin.copy_into_slice(&mut buf[..len as usize]);
    let bytes = &buf[..len as usize];

    const SCHEME: &[u8] = b"https://";
    if bytes.len() <= SCHEME.len() || &bytes[..SCHEME.len()] != SCHEME {
        e.panic_with_error(Error::InvalidOrigin);
    }
    let host = &bytes[SCHEME.len()..];
    if host.is_empty() {
        e.panic_with_error(Error::InvalidOrigin);
    }
    for b in bytes.iter() {
        // Reject whitespace, query, fragment, and trailing slash.
        if b.is_ascii_whitespace() || *b == b'?' || *b == b'#' {
            e.panic_with_error(Error::InvalidOrigin);
        }
    }
    if bytes[bytes.len() - 1] == b'/' {
        e.panic_with_error(Error::InvalidOrigin);
    }
}

#[contractimpl]
impl Account {
    pub fn __constructor(e: Env, config: Config) {
        if config.keys.len() == 0 || config.keys.len() > 10 {
            e.panic_with_error(Error::InvalidKeyCount);
        }
        if config.threshold == 0 || config.threshold > config.keys.len() {
            e.panic_with_error(Error::InvalidThreshold);
        }
        validate_rp_id_hash(&e, &config.rp_id_hash);
        validate_origin(&e, &config.origin);
        for i in 0..config.keys.len() {
            let key = config.keys.get(i).unwrap();
            validate_public_key(&e, &key);
            for j in 0..i {
                if config.keys.get(i) == config.keys.get(j) {
                    e.panic_with_error(Error::InvalidPublicKey);
                }
            }
        }
        e.storage().instance().set(&Key::Config, &config);
        touch(&e);
    }

    /// Full passkey quorum is required. Replacing a key resets its explicit budget.
    pub fn add_session(e: Env, public_key: BytesN<32>, session: Session) {
        e.current_contract_address().require_auth();
        assert!(session.per_transfer > 0 && session.remaining >= session.per_transfer);
        assert!(session.expires_at > e.ledger().timestamp());
        e.storage()
            .instance()
            .set(&Key::Session(public_key.clone()), &session);
        e.events().publish(
            (symbol_short!("session"), symbol_short!("set"), public_key),
            session,
        );
        touch(&e);
    }
    pub fn revoke_session(e: Env, public_key: BytesN<32>) {
        e.current_contract_address().require_auth();
        e.storage()
            .instance()
            .remove(&Key::Session(public_key.clone()));
        e.events().publish(
            (
                symbol_short!("session"),
                symbol_short!("revoked"),
                public_key,
            ),
            (),
        );
        touch(&e);
    }
    /// A passkey quorum may freeze all sessions without blocking passkey transfers or revocation.
    pub fn set_sessions_paused(e: Env, paused: bool) {
        e.current_contract_address().require_auth();
        e.storage().instance().set(&Key::SessionsPaused, &paused);
        e.events()
            .publish((symbol_short!("session"), symbol_short!("paused")), paused);
        touch(&e);
    }
    pub fn sessions_paused(e: Env) -> bool {
        e.storage()
            .instance()
            .get(&Key::SessionsPaused)
            .unwrap_or(false)
    }
    /// Lets clients discover the credential order, quorum, RP binding and origin.
    pub fn config(e: Env) -> Config {
        e.storage().instance().get(&Key::Config).unwrap()
    }
    pub fn session(e: Env, public_key: BytesN<32>) -> Option<Session> {
        e.storage().instance().get(&Key::Session(public_key))
    }
}

#[contractimpl]
impl CustomAccountInterface for Account {
    type Signature = Signature;
    type Error = Error;
    fn __check_auth(
        e: Env,
        signature_payload: soroban_sdk::crypto::Hash<32>,
        signature: Signature,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Error> {
        let signature_payload: BytesN<32> = signature_payload.into();
        match signature {
            Signature::Passkeys(assertions) => {
                let config: Config = e
                    .storage()
                    .instance()
                    .get(&Key::Config)
                    .ok_or(Error::Invalid)?;
                if assertions.len() < config.threshold || assertions.len() > config.keys.len() {
                    return Err(Error::Invalid);
                }
                let mut seen = Vec::new(&e);
                for assertion in assertions {
                    if seen.contains(&assertion.key_index) {
                        return Err(Error::Invalid);
                    }
                    let key = config.keys.get(assertion.key_index).ok_or(Error::Invalid)?;
                    verify_assertion(
                        &e,
                        &key,
                        &config.rp_id_hash,
                        &config.origin,
                        &signature_payload,
                        &assertion,
                    );
                    seen.push_back(assertion.key_index);
                }
            }
            Signature::Session(public_key, sig) => {
                if Self::sessions_paused(e.clone()) {
                    return Err(Error::Paused);
                }
                let storage_key = Key::Session(public_key.clone());
                let mut session: Session = e
                    .storage()
                    .instance()
                    .get(&storage_key)
                    .ok_or(Error::Invalid)?;
                if e.ledger().timestamp() >= session.expires_at {
                    return Err(Error::Expired);
                }
                if auth_contexts.is_empty() {
                    return Err(Error::Scope);
                }
                let mut total = 0i128;
                for context in auth_contexts {
                    let call = match context {
                        Context::Contract(c) => c,
                        _ => return Err(Error::Scope),
                    };
                    if call.contract != session.token
                        || call.fn_name != Symbol::new(&e, "transfer")
                        || call.args.len() != 3
                    {
                        return Err(Error::Scope);
                    }
                    let from = Address::try_from_val(&e, &call.args.get(0).unwrap())
                        .map_err(|_| Error::Scope)?;
                    let to = Address::try_from_val(&e, &call.args.get(1).unwrap())
                        .map_err(|_| Error::Scope)?;
                    let amount = i128::try_from_val(&e, &call.args.get(2).unwrap())
                        .map_err(|_| Error::Scope)?;
                    if session
                        .recipient
                        .as_ref()
                        .is_some_and(|recipient| *recipient != to)
                    {
                        return Err(Error::Scope);
                    }
                    if from != e.current_contract_address() {
                        return Err(Error::Scope);
                    }
                    if amount <= 0 || amount > session.per_transfer {
                        return Err(Error::Limit);
                    }
                    total = total.checked_add(amount).ok_or(Error::Limit)?;
                }
                if total > session.remaining {
                    return Err(Error::Limit);
                }
                e.crypto()
                    .ed25519_verify(&public_key, &Bytes::from(signature_payload), &sig);
                session.remaining -= total;
                e.storage().instance().set(&storage_key, &session);
                e.events().publish(
                    (symbol_short!("session"), symbol_short!("spent"), public_key),
                    (total, session.remaining),
                );
            }
        }
        touch(&e);
        Ok(())
    }
}
#[cfg(test)]
mod test;
