#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    vec, Address, Env, IntoVal, Val,
};
use stellar_default_impl_macro::default_impl;
use stellar_fungible::{Base, FungibleToken};

use crate::{Router, RouterClient, MAX_ARG_BYTES, MAX_INVOCATIONS};

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
        Base::mint(e, &owner, initial_supply);
    }

    pub fn mint(e: &Env, to: Address, amount: i128) {
        Base::mint(e, &to, amount);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for TokenContract {
    type ContractType = Base;
}

fn create_token_contract<'a>(e: &Env, owner: &Address) -> TokenContractClient<'a> {
    let address = e.register(TokenContract, (owner, 10_000i128));
    TokenContractClient::new(e, &address)
}

#[test]
fn test_exec() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_a = Address::generate(&e);
    let token_a_client = create_token_contract(&e, &token_a);

    let token_b = Address::generate(&e);
    let token_b_client = create_token_contract(&e, &token_b);

    let caller = Address::generate(&e);
    let recipient = Address::generate(&e);

    token_a_client.mint(&caller, &100);
    token_b_client.mint(&caller, &200);

    let contract_id = e.register(Router, ());
    let client = RouterClient::new(&e, &contract_id);

    e.set_auths(&[]);

    let transfer_invocations = vec![
        &e,
        (
            token_a_client.address.clone(),
            symbol_short!("transfer"),
            (caller.clone(), recipient.clone(), 50_i128).into_val(&e),
        ),
        (
            token_b_client.address.clone(),
            symbol_short!("transfer"),
            (caller.clone(), recipient.clone(), 75_i128).into_val(&e),
        ),
    ];

    e.mock_auths(&[MockAuth {
        address: &caller,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "exec",
            args: (caller.clone(), transfer_invocations.clone()).into_val(&e),
            sub_invokes: &[
                MockAuthInvoke {
                    contract: &token_a_client.address,
                    fn_name: "transfer",
                    args: (caller.clone(), recipient.clone(), 50_i128).into_val(&e),
                    sub_invokes: &[],
                },
                MockAuthInvoke {
                    contract: &token_b_client.address,
                    fn_name: "transfer",
                    args: (caller.clone(), recipient.clone(), 75_i128).into_val(&e),
                    sub_invokes: &[],
                },
            ],
        },
    }]);

    let transfer_results = client.exec(&caller, &transfer_invocations);

    assert_eq!(transfer_results.len(), 2);

    assert_eq!(token_a_client.balance(&caller), 50);
    assert_eq!(token_a_client.balance(&recipient), 50);
    assert_eq!(token_b_client.balance(&caller), 125);
    assert_eq!(token_b_client.balance(&recipient), 75);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_exec_unauthorized() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_a = Address::generate(&e);
    let token_a_client = create_token_contract(&e, &token_a);

    let token_b = Address::generate(&e);
    let token_b_client = create_token_contract(&e, &token_b);

    let caller = Address::generate(&e);
    let recipient = Address::generate(&e);

    token_a_client.mint(&caller, &100);
    token_b_client.mint(&caller, &200);

    let contract_id = e.register(Router, ());
    let client = RouterClient::new(&e, &contract_id);

    e.set_auths(&[]);

    let transfer_invocations: soroban_sdk::Vec<(
        Address,
        soroban_sdk::Symbol,
        soroban_sdk::Vec<Val>,
    )> = vec![
        &e,
        (
            token_a_client.address.clone(),
            symbol_short!("transfer"),
            vec![
                &e,
                caller.into_val(&e),
                recipient.into_val(&e),
                50_i128.into_val(&e),
            ],
        ),
        (
            token_b_client.address.clone(),
            symbol_short!("transfer"),
            vec![
                &e,
                caller.into_val(&e),
                recipient.into_val(&e),
                75_i128.into_val(&e),
            ],
        ),
    ];

    client.exec(&caller, &transfer_invocations);
}

#[test]
fn test_exec_atomicity() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_a = Address::generate(&e);
    let token_a_client = create_token_contract(&e, &token_a);

    let token_b = Address::generate(&e);
    let token_b_client = create_token_contract(&e, &token_b);

    let caller = Address::generate(&e);
    let recipient = Address::generate(&e);

    token_a_client.mint(&caller, &100);
    token_b_client.mint(&caller, &50);

    let contract_id = e.register(Router, ());
    let client = RouterClient::new(&e, &contract_id);

    // First transfer should succeed, second should fail due to insufficient balance
    let transfer_invocations = vec![
        &e,
        (
            token_a_client.address.clone(),
            symbol_short!("transfer"),
            (caller.clone(), recipient.clone(), 40_i128).into_val(&e),
        ),
        (
            token_b_client.address.clone(),
            symbol_short!("transfer"),
            (caller.clone(), recipient.clone(), 75_i128).into_val(&e),
        ), // This will fail
    ];

    let result = client.try_exec(&caller, &transfer_invocations);
    assert!(result.is_err());

    assert_eq!(token_a_client.balance(&caller), 100);
    assert_eq!(token_a_client.balance(&recipient), 0);
}

#[test]
fn test_exec_max_invocations_succeeds() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token = Address::generate(&e);
    let token_client = create_token_contract(&e, &token);

    let caller = Address::generate(&e);
    let recipient = Address::generate(&e);

    token_client.mint(&caller, &(MAX_INVOCATIONS as i128));

    let contract_id = e.register(Router, ());
    let client = RouterClient::new(&e, &contract_id);

    let mut invocations: soroban_sdk::Vec<(
        Address,
        soroban_sdk::Symbol,
        soroban_sdk::Vec<Val>,
    )> = vec![&e];
    for _ in 0..MAX_INVOCATIONS {
        invocations.push_back((
            token_client.address.clone(),
            symbol_short!("transfer"),
            (caller.clone(), recipient.clone(), 1_i128).into_val(&e),
        ));
    }

    let results = client.exec(&caller, &invocations);
    assert_eq!(results.len(), MAX_INVOCATIONS);

    assert_eq!(token_client.balance(&caller), 0);
    assert_eq!(token_client.balance(&recipient), MAX_INVOCATIONS as i128);
}

#[test]
fn test_exec_too_many_invocations() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token = Address::generate(&e);
    let token_client = create_token_contract(&e, &token);

    let caller = Address::generate(&e);
    let recipient = Address::generate(&e);

    token_client.mint(&caller, &((MAX_INVOCATIONS + 1) as i128));

    let contract_id = e.register(Router, ());
    let client = RouterClient::new(&e, &contract_id);

    let mut invocations: soroban_sdk::Vec<(
        Address,
        soroban_sdk::Symbol,
        soroban_sdk::Vec<Val>,
    )> = vec![&e];
    for _ in 0..(MAX_INVOCATIONS + 1) {
        invocations.push_back((
            token_client.address.clone(),
            symbol_short!("transfer"),
            (caller.clone(), recipient.clone(), 1_i128).into_val(&e),
        ));
    }

    let result = client.try_exec(&caller, &invocations);
    assert_eq!(result, Err(Ok(crate::RouterError::TooManyInvocations)));

    // No dispatch should have occurred.
    assert_eq!(token_client.balance(&caller), (MAX_INVOCATIONS + 1) as i128);
    assert_eq!(token_client.balance(&recipient), 0);
}

#[test]
fn test_exec_argument_too_large() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token = Address::generate(&e);
    let token_client = create_token_contract(&e, &token);

    let caller = Address::generate(&e);
    let recipient = Address::generate(&e);

    token_client.mint(&caller, &100);

    let contract_id = e.register(Router, ());
    let client = RouterClient::new(&e, &contract_id);

    // Build an argument payload whose encoded size exceeds MAX_ARG_BYTES.
    let big = soroban_sdk::Bytes::from_array(&e, &[0u8; 1]);
    let mut big_bytes = soroban_sdk::Bytes::new(&e);
    // Each byte contributes at least one byte to the encoded size; pad well past the limit.
    for _ in 0..(MAX_ARG_BYTES + 1) {
        big_bytes.extend_from_array(&[0u8]);
    }
    let _ = big;

    let invocations: soroban_sdk::Vec<(
        Address,
        soroban_sdk::Symbol,
        soroban_sdk::Vec<Val>,
    )> = vec![
        &e,
        (
            token_client.address.clone(),
            symbol_short!("transfer"),
            vec![&e, big_bytes.into_val(&e)],
        ),
    ];

    let result = client.try_exec(&caller, &invocations);
    assert_eq!(result, Err(Ok(crate::RouterError::ArgumentTooLarge)));

    // No dispatch should have occurred.
    assert_eq!(token_client.balance(&caller), 100);
    assert_eq!(token_client.balance(&recipient), 0);
}
