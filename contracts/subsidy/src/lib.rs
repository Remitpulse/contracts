//! RemitPulse token subsidy vault. Network fees are sponsored by an off-chain fee-bump payer.
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env,
};
#[contracttype]
#[derive(Clone)]
enum Key {
    Admin,
    ActiveEpoch,
    Used(u32, BytesN<32>),
}
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidAmount = 1,
    AlreadyReimbursed = 2,
}
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
    /// Admin-authorized epoch rotation. Markers from prior epochs are no longer
    /// consulted, bounding replay-marker growth to the active epoch.
    pub fn rotate_epoch(e: Env) -> u32 {
        let admin: Address = e.storage().instance().get(&Key::Admin).unwrap();
        admin.require_auth();
        let next = Self::active_epoch(e.clone()) + 1;
        e.storage().instance().set(&Key::ActiveEpoch, &next);
        e.storage().instance().extend_ttl(120_960, 518_400);
        next
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
        e.storage().instance().set(&Key::Used(epoch, id.clone()), &true);
        token::Client::new(&e, &asset).transfer(&e.current_contract_address(), &recipient, &amount);
        e.events()
            .publish((symbol_short!("subsidy"), id), (asset, recipient, amount));
        e.storage().instance().extend_ttl(120_960, 518_400);
        Ok(())
    }
}
#[cfg(test)]
mod test;
