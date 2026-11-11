//! RemitPulse token subsidy vault. Network fees are sponsored by an off-chain fee-bump payer.
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, BytesN,
    Env, Vec,
};
#[contracttype]
#[derive(Clone)]
enum Key {
    Admin,
    ActiveEpoch,
    Used(u32, BytesN<32>),
    EpochIds,
}
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidAmount = 1,
    AlreadyReimbursed = 2,
    InsufficientBalance = 3,
    EpochFull = 4,
    EpochExhausted = 5,
}
/// Keep instance storage bounded; reconcile and rotate before reaching this cap.
pub const MAX_REIMBURSEMENTS: u32 = 128;
#[contract]
pub struct Subsidy;
#[contractimpl]
impl Subsidy {
    pub fn __constructor(e: Env, admin: Address) {
        e.storage().instance().set(&Key::Admin, &admin);
        e.storage().instance().set(&Key::ActiveEpoch, &0u32);
        e.storage().instance().extend_ttl(120_960, 518_400);
    }
    /// Admin-authorized, exactly-once token reimbursement. Fund the vault by SEP-41 transfer.
    /// Each token (USDC/EURC/ARST/BRLA) has its own balance and native base-unit amounts.
    pub fn admin(e: Env) -> Address {
        e.storage().instance().get(&Key::Admin).unwrap()
    }
    /// The current reimbursement epoch. Replay markers are scoped to this epoch.
    pub fn active_epoch(e: Env) -> u32 {
        e.storage().instance().get(&Key::ActiveEpoch).unwrap_or(0)
    }
    /// Admin-authorized epoch rotation removes active markers and their ID index.
    /// Replay rejection is deliberately scoped to one bounded epoch.
    pub fn rotate_epoch(e: Env) -> Result<u32, Error> {
        let admin: Address = e.storage().instance().get(&Key::Admin).unwrap();
        admin.require_auth();
        let epoch = Self::active_epoch(e.clone());
        let next = epoch.checked_add(1).ok_or(Error::EpochExhausted)?;
        let ids: Vec<BytesN<32>> = e
            .storage()
            .instance()
            .get(&Key::EpochIds)
            .unwrap_or(vec![&e]);
        for id in ids {
            e.storage().instance().remove(&Key::Used(epoch, id));
        }
        e.storage().instance().remove(&Key::EpochIds);
        e.storage().instance().set(&Key::ActiveEpoch, &next);
        e.storage().instance().extend_ttl(120_960, 518_400);
        e.events().publish((symbol_short!("epoch"),), next);
        Ok(next)
    }
    pub fn is_reimbursed(e: Env, id: BytesN<32>) -> bool {
        let epoch = Self::active_epoch(e.clone());
        e.storage().instance().has(&Key::Used(epoch, id))
    }
    pub fn reimburse(
        e: Env,
        id: BytesN<32>,
        asset: Address,
        recipient: Address,
        amount: i128,
    ) -> Result<(), Error> {
        let admin: Address = e.storage().instance().get(&Key::Admin).unwrap();
        admin.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        // Replay markers are scoped to the active epoch; rotation bounds their growth.
        let epoch = Self::active_epoch(e.clone());
        if e.storage().instance().has(&Key::Used(epoch, id.clone())) {
            return Err(Error::AlreadyReimbursed);
        }
        let mut ids: Vec<BytesN<32>> = e
            .storage()
            .instance()
            .get(&Key::EpochIds)
            .unwrap_or(vec![&e]);
        if ids.len() >= MAX_REIMBURSEMENTS {
            return Err(Error::EpochFull);
        }
        // Return a stable contract error rather than relying on the token
        // implementation's failure when the vault is underfunded.
        if token::Client::new(&e, &asset).balance(&e.current_contract_address()) < amount {
            return Err(Error::InsufficientBalance);
        }
        e.storage()
            .instance()
            .set(&Key::Used(epoch, id.clone()), &true);
        ids.push_back(id.clone());
        e.storage().instance().set(&Key::EpochIds, &ids);
        token::Client::new(&e, &asset).transfer(&e.current_contract_address(), &recipient, &amount);
        e.events()
            .publish((symbol_short!("subsidy"), id), (asset, recipient, amount));
        e.storage().instance().extend_ttl(120_960, 518_400);
        Ok(())
    }
}
#[cfg(test)]
mod test;
