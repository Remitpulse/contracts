//! Proofs generated using https://github.com/philipliu/soroban-merkle-airdrop

#![cfg(test)]

use crate::{AirdropContract, AirdropContractClient, AirdropError as Error};
use hex_literal::hex;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{
        storage::Instance as _, Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke,
    },
    vec, Address, BytesN, Env, IntoVal, TryFromVal,
};
use stellar_default_impl_macro::default_impl;
use stellar_fungible::{Base, FungibleToken};

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

/// Number of ledgers after the current one at which the claim window closes.
const CLAIM_DEADLINE_OFFSET: u32 = 100;

fn make_args(
    e: &Env,
    hash_bytes: [u8; 32],
    token: Address,
    admin: Address,
    funder: Address,
) -> (BytesN<32>, Address, Address, Address, u32) {
    let root_hash = BytesN::from_array(e, &hash_bytes);
    let claim_deadline = e.ledger().sequence() + CLAIM_DEADLINE_OFFSET;

    (root_hash, token, admin, funder, claim_deadline)
}

/// Advance the ledger past the claim deadline configured by `make_args`.
fn advance_past_deadline(e: &Env) {
    e.ledger()
        .set_sequence_number(e.ledger().sequence() + CLAIM_DEADLINE_OFFSET + 1);
}

fn hex_to_bytes(e: &Env, hash_bytes: [u8; 32]) -> BytesN<32> {
    BytesN::from_array(e, &hash_bytes)
}

#[test]
fn test_valid_claim() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let constructor_args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );

    let contract_id = e.register(AirdropContract, constructor_args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    let receiver = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount = 100;
    let proofs = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        hex_to_bytes(
            &e,
            hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];

    e.set_auths(&[]);

    client
        .mock_auths(&[MockAuth {
            address: &receiver,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "claim",
                args: (3_u32, receiver.clone(), amount, proofs.clone()).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .claim(&3_u32, &receiver, &amount, &proofs);
    assert_eq!(token_client.balance(&receiver), 100);
    assert_eq!(token_client.balance(&contract_id), 900);
}

#[test]
fn test_valid_claim_no_auth() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let constructor_args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );

    let contract_id = e.register(AirdropContract, constructor_args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    let receiver = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount = 100;
    let proofs = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        hex_to_bytes(
            &e,
            hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];

    e.set_auths(&[]);

    let res = client.try_claim(&3_u32, &receiver, &amount, &proofs);
    assert!(res.is_err());
}

#[test]
fn test_double_claim() {
    let e: Env = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    let receiver = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount: i128 = 100;
    let proofs = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        hex_to_bytes(
            &e,
            hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];

    client.claim(&3_u32, &receiver, &amount, &proofs);
    let second_claim = client.try_claim(&3_u32, &receiver, &amount, &proofs);

    assert!(second_claim.is_err());
}

#[test]
fn test_claimed_not_reset() {
    let e: Env = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("9ecccb575ce934ab36a6db174e9f521137c942422b76332b047b49f5a1a58048"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    let receiver_1 = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount_1: i128 = 100;
    let proofs_1 = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("cd9bbfb141e8c63b620238d79aabfbe5eaf16309874b3f32fc443b4f477c9b2f"),
        ),
        hex_to_bytes(
            &e,
            hex!("ae7ed9c150e2d582d1db0a32dc7370c00a22405324e5b5f1c9272e57274a08f4"),
        ),
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
    ];

    let receiver_2 = Address::from_str(
        &e,
        "CCAYN4HGXBYMAREFANQKKRNCIPLXYGXT7OVXDXG6APXBGKJPKARAOHAK",
    );
    let amount_2: i128 = 100;
    let proofs_2 = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("bab7bc2e36db8910a5e047989f1bfb6791bb8a2d3b3218fd363969294aaac83e"),
        ),
        hex_to_bytes(
            &e,
            hex!("c8b6359bcd036ed19bff1e307c7f0eeb410ec193a5a4647f7cf36fdba86af070"),
        ),
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
    ];

    client.claim(&3_u32, &receiver_1, &amount_1, &proofs_1);
    client.claim(&4_u32, &receiver_2, &amount_2, &proofs_2);
    let second_receiver_1_claim = client.try_claim(&3_u32, &receiver_1, &amount_2, &proofs_2);

    assert!(second_receiver_1_claim.is_err());
}

#[test]
fn test_bad_claim() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    let receiver = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount = 100000; // This is a different amount
    let proofs = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        hex_to_bytes(
            &e,
            hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];

    let claim = client.try_claim(&3_u32, &receiver, &amount, &proofs);

    assert!(claim.is_err());
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn test_recover_unclaimed_to_funder() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_owner = Address::generate(&e);
    let admin = Address::generate(&e);
    let funder = Address::generate(&e);
    let token_client = create_token_contract(&e, &token_owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        admin.clone(),
        funder.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);

    token_client.transfer(&token_owner, &contract_id, &1000);

    advance_past_deadline(&e);

    e.set_auths(&[]);

    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "recover_unclaimed",
                args: ().into_val(&e),
                sub_invokes: &[],
            },
        }])
        .recover_unclaimed();

    assert_eq!(token_client.balance(&funder), 1000);
    assert_eq!(token_client.balance(&admin), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
    assert!(client.is_ended());

    let receiver = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount = 100;
    let proofs = vec![&e];

    client.claim(&3_u32, &receiver, &amount, &proofs);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn test_recover_unclaimed_after_ended() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    advance_past_deadline(&e);

    client.recover_unclaimed();
    client.recover_unclaimed();
}

#[test]
fn test_recover_unclaimed_no_admin_auth() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_owner = Address::generate(&e);
    let admin = Address::generate(&e);
    let funder = Address::generate(&e);
    let random_caller = Address::generate(&e);
    let token_client = create_token_contract(&e, &token_owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        admin.clone(),
        funder.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&token_owner, &contract_id, &1000);

    advance_past_deadline(&e);

    e.set_auths(&[]);

    let result = client.try_recover_unclaimed();
    assert!(result.is_err());

    let result_with_funder = client
        .mock_auths(&[MockAuth {
            address: &funder,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "recover_unclaimed",
                args: ().into_val(&e),
                sub_invokes: &[],
            },
        }])
        .try_recover_unclaimed();
    assert!(result_with_funder.is_err());

    let result_with_random = client
        .mock_auths(&[MockAuth {
            address: &random_caller,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "recover_unclaimed",
                args: ().into_val(&e),
                sub_invokes: &[],
            },
        }])
        .try_recover_unclaimed();
    assert!(result_with_random.is_err());
}

#[test]
fn test_recover_unclaimed_before_deadline_fails() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_owner = Address::generate(&e);
    let admin = Address::generate(&e);
    let funder = Address::generate(&e);
    let token_client = create_token_contract(&e, &token_owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        admin.clone(),
        funder.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&token_owner, &contract_id, &1000);

    let result = client.try_recover_unclaimed();
    assert_eq!(
        result,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::ClaimWindowOpen as u32
        )))
    );

    // Funds remain in the contract and the airdrop is still claimable.
    assert_eq!(token_client.balance(&contract_id), 1000);
    assert_eq!(token_client.balance(&funder), 0);
    assert!(!client.is_ended());
}

#[test]
fn test_recover_unclaimed_after_deadline_succeeds() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let token_owner = Address::generate(&e);
    let admin = Address::generate(&e);
    let funder = Address::generate(&e);
    let token_client = create_token_contract(&e, &token_owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        admin.clone(),
        funder.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&token_owner, &contract_id, &1000);

    advance_past_deadline(&e);

    client.recover_unclaimed();
    let events = e.events().all();

    assert_eq!(token_client.balance(&funder), 1000);
    assert_eq!(token_client.balance(&contract_id), 0);
    assert!(client.is_ended());

    let recovered_event = events
        .iter()
        .find(|(_, topics, _)| {
            topics
                .get(0)
                .map(|t| {
                    soroban_sdk::Symbol::try_from_val(&e, &t).ok()
                        == Some(soroban_sdk::Symbol::new(&e, "recovered"))
                })
                .unwrap_or(false)
        })
        .expect("recovered event should be emitted");
    let _ = recovered_event;
}

#[test]
fn test_instance_ttl_extended_on_lifecycle() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let mut args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        owner.clone(),
        owner.clone(),
    );
    args.4 = e.ledger().sequence() + 2 * crate::DAY_IN_LEDGERS + 2;
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);
    token_client.transfer(&owner, &contract_id, &1000);

    // Renew token balances too: OZ mint/transfer creates entries at default TTL.
    token_client.balance(&contract_id);
    token_client.balance(&owner);
    // Maintain the token fixture separately while testing airdrop instance renewal.
    e.as_contract(&token_client.address, || {
        e.storage().instance().extend_ttl(120_960, 518_400)
    });

    // TTL is set at construction.
    let ttl_after_construction = e.as_contract(&contract_id, || e.storage().instance().get_ttl());
    assert!(ttl_after_construction > 0);

    // Advance the ledger so the TTL has a chance to decay, then claim.
    e.ledger()
        .set_sequence_number(e.ledger().sequence() + crate::DAY_IN_LEDGERS + 1);

    let receiver = Address::from_str(
        &e,
        "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
    );
    let amount = 100;
    let proofs = vec![
        &e,
        hex_to_bytes(
            &e,
            hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7"),
        ),
        hex_to_bytes(
            &e,
            hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4"),
        ),
    ];

    client.claim(&3_u32, &receiver, &amount, &proofs);

    let ttl_after_claim = e.as_contract(&contract_id, || e.storage().instance().get_ttl());
    assert!(ttl_after_claim >= ttl_after_construction);

    // Advance past the deadline and recover; TTL should be renewed again.
    e.ledger().set_sequence_number(client.claim_deadline());
    client.recover_unclaimed();

    let ttl_after_recovery = e.as_contract(&contract_id, || e.storage().instance().get_ttl());
    assert!(ttl_after_recovery >= ttl_after_claim);
}

#[test]
fn claims_close_at_deadline_before_proof_or_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = create_token_contract(&e, &admin);
    let args = make_args(
        &e,
        [1; 32],
        token.address.clone(),
        admin.clone(),
        admin.clone(),
    );
    let id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &id);
    e.ledger().set_sequence_number(client.claim_deadline());
    assert_eq!(
        client.try_claim(&0, &admin, &1, &vec![&e]),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::ClaimWindowClosed as u32
        )))
    );
    assert!(!client.is_claimed(&0));
}

/// Reconstruct the historical four-recipient fixture using SDK XDR and host SHA-256.
/// Data lineage: philipliu/soroban-merkle-airdrop@b611ea8, merkle/receivers.json;
/// the same proof is present in the attributed OpenZeppelin example. No upstream
/// generator code is vendored here; redistribution rights remain separately reviewed.
#[test]
fn historical_fixture_reconstructs_and_binds_every_leaf_field() {
    use soroban_sdk::{xdr::ToXdr, Bytes};
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();
    let receivers = [
        (
            "GD5RUZEO3ZCW6UX6Y4FRHKC7ZWWKTKUOPCQKYKYPCF2K7M7AD7J2URPW",
            100_000_000i128,
        ),
        (
            "GAQ32GZG3X5AH2QU66WZ6HMU44UVXZOC4GZNH57KB4Y2XQBKVAK4FMJE",
            100_000_000,
        ),
        (
            "GBBGPC7OBPPDH7VL5MYGROGPIG7VXXMS5KD34YOVA7HYRLNLMU45MJ62",
            100_000_000,
        ),
        (
            "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX",
            100,
        ),
    ];
    let mut leaves = [[0u8; 32]; 4];
    for (index, (address, amount)) in receivers.iter().enumerate() {
        let encoded = crate::Receiver {
            index: index as u32,
            address: Address::from_str(&e, address),
            amount: *amount,
        }
        .to_xdr(&e);
        leaves[index] = e.crypto().sha256(&encoded).to_array();
    }
    let target = leaves[3];
    leaves.sort_unstable();
    let position = leaves.iter().position(|leaf| *leaf == target).unwrap();
    let merge = |a: [u8; 32], b: [u8; 32]| {
        let (first, second) = if a < b { (a, b) } else { (b, a) };
        let mut data = Bytes::from_array(&e, &first);
        data.extend_from_array(&second);
        e.crypto().sha256(&data).to_array()
    };
    let parents = [merge(leaves[0], leaves[1]), merge(leaves[2], leaves[3])];
    let root = merge(parents[0], parents[1]);
    assert_eq!(
        root,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722")
    );
    let sibling = leaves[position ^ 1];
    let other_parent = parents[(position / 2) ^ 1];
    assert_eq!(
        sibling,
        hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7")
    );
    assert_eq!(
        other_parent,
        hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4")
    );

    let admin = Address::generate(&e);
    let token = create_token_contract(&e, &admin);
    let id = e.register(
        AirdropContract,
        make_args(
            &e,
            root,
            token.address.clone(),
            admin.clone(),
            admin.clone(),
        ),
    );
    let client = AirdropContractClient::new(&e, &id);
    token.transfer(&admin, &id, &1000);
    let recipient = Address::from_str(&e, receivers[3].0);
    let proof = vec![
        &e,
        BytesN::from_array(&e, &sibling),
        BytesN::from_array(&e, &other_parent),
    ];
    assert!(client.try_claim(&4, &recipient, &100, &proof).is_err());
    assert!(client.try_claim(&3, &admin, &100, &proof).is_err());
    assert!(client.try_claim(&3, &recipient, &101, &proof).is_err());
    assert!(!client.is_claimed(&3));
    client.claim(&3, &recipient, &100, &proof);
    assert_eq!(token.balance(&recipient), 100);
    assert!(client.is_claimed(&3));
    assert!(client.try_claim(&3, &recipient, &100, &proof).is_err());
}
