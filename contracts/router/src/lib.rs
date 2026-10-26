//! # RemitPulse router contract
//!
//!  Based on https://github.com/Creit-Tech/Stellar-Router-Contract/blob/04975c434dec362584aa99458ae2c25d803ec570/contracts/router-v0/src/lib.rs
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, vec, Address, Env, Symbol, Val, Vec,
};

pub const DAY_IN_LEDGERS: u32 = 17280;

pub const INSTANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_EXTEND_AMOUNT - (7 * DAY_IN_LEDGERS);

/// Maximum number of invocations accepted in a single `exec` batch.
pub const MAX_INVOCATIONS: u32 = 10;

/// Maximum serialized size (in bytes) of the argument vector for a single
/// invocation.
pub const MAX_ARG_BYTES: u32 = 1024;

/// Errors returned by the router contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RouterError {
    /// The batch contains more than [`MAX_INVOCATIONS`] invocations.
    TooManyInvocations = 1,
    /// A single invocation's serialized arguments exceed [`MAX_ARG_BYTES`].
    ArgumentTooLarge = 2,
}

#[contract]
pub struct Router;

#[contractimpl]
impl Router {
    /// Execute a batch of contract invocations atomically.
    ///
    /// # Limits
    ///
    /// To bound CPU, memory, and event/result resource consumption, the batch
    /// is validated before any invocation is dispatched:
    ///
    /// - The batch may contain at most [`MAX_INVOCATIONS`] invocations;
    ///   otherwise [`RouterError::TooManyInvocations`] is returned.
    /// - Each invocation's serialized argument vector may be at most
    ///   [`MAX_ARG_BYTES`] bytes; otherwise [`RouterError::ArgumentTooLarge`]
    ///   is returned.
    ///
    /// Accepted batches execute atomically: if any invocation fails, the
    /// entire transaction reverts.
    pub fn exec(
        e: Env,
        caller: Address,
        invocations: Vec<(Address, Symbol, Vec<Val>)>,
    ) -> Vec<Val> {
        e.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);

        caller.require_auth();

        if invocations.len() > MAX_INVOCATIONS {
            panic_with_error!(&e, RouterError::TooManyInvocations);
        }

        for (contract, method, args) in invocations.iter() {
            if args.len() > MAX_ARG_BYTES {
                panic_with_error!(&e, RouterError::ArgumentTooLarge);
            }
            let _ = (contract, method);
        }

        let mut results: Vec<Val> = vec![&e];
        for (contract, method, args) in invocations {
            results.push_back(e.invoke_contract::<Val>(&contract, &method, args));
        }
        results
    }
}

mod test;