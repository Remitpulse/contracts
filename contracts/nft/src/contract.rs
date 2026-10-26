use crate::{
    errors::NonFungibleTokenContractError,
    types::{DataKey, TokenData, TokenMetadata},
};
use soroban_sdk::{contract, contractimpl, panic_with_error, vec, Address, Env, String, Vec};
use stellar_non_fungible::{
    burnable::NonFungibleBurnable, Base, NFTStorageKey, NonFungibleToken, NonFungibleTokenError,
};

#[contract]
pub struct Contract;
pub const WEEK_OF_LEDGERS: u32 = 60 * 60 * 24 / 5 * 7; // assumes 5 second ledger close times

// Storage TTL maintenance constants (in ledgers).
pub const INSTANCE_TTL_THRESHOLD: u32 = WEEK_OF_LEDGERS;
pub const INSTANCE_TTL_EXTEND_TO: u32 = WEEK_OF_LEDGERS * 2;
pub const PERSISTENT_TTL_THRESHOLD: u32 = WEEK_OF_LEDGERS;
pub const PERSISTENT_TTL_EXTEND_TO: u32 = WEEK_OF_LEDGERS * 2;

// Bounds for bulk operations and pagination.
pub const MAX_BULK_MINT: u32 = 50;
pub const MAX_OWNER_TOKENS_PAGE: u32 = 100;

fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

fn extend_nft_persistent_ttl(env: &Env, key: &NFTStorageKey<u32>) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

#[contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, owner: Address, max_supply: u32, metadata: TokenMetadata) {
        extend_instance_ttl(env);
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage()
            .instance()
            .set(&DataKey::MaxSupply, &max_supply);
        env.storage().instance().set(&DataKey::TotalMinted, &0u32);

        Base::set_metadata(env, metadata.base_uri, metadata.name, metadata.symbol);
    }

    pub fn set_metadata_uri(env: &Env, base_uri: String) {
        extend_instance_ttl(env);
        Self::only_owner(env);

        let metadata = Base::get_metadata(env);

        Base::set_metadata(env, base_uri, metadata.name, metadata.symbol);
    }

    pub fn total_supply(env: &Env) -> u32 {
        extend_instance_ttl(env);
        env.storage().instance().get(&DataKey::TotalMinted).unwrap()
    }

    pub fn get_max_supply(env: &Env) -> u32 {
        extend_instance_ttl(env);
        env.storage().instance().get(&DataKey::MaxSupply).unwrap()
    }

    fn only_owner(env: &Env) {
        let owner: Address = env
            .storage()
            .instance()
            .get(&DataKey::Owner)
            .unwrap_or_else(|| panic_with_error!(env, NonFungibleTokenContractError::UnsetOwner));

        owner.require_auth();
    }

    fn set_token_data(env: &Env, token_id: u32, data: TokenData) {
        let key = DataKey::TokenData(token_id);
        env.storage().persistent().set(&key, &data);
        extend_persistent_ttl(env, &key);
    }

    pub fn get_token_data(env: &Env, token_id: u32) -> TokenData {
        extend_instance_ttl(env);
        let key = DataKey::TokenData(token_id);
        let data = env.storage().persistent().get(&key).unwrap_or_else(|| {
            panic_with_error!(env, NonFungibleTokenContractError::UnsetTokenData)
        });
        extend_persistent_ttl(env, &key);
        data
    }

    /// Returns the complete list of token IDs owned by `owner`.
    ///
    /// Deprecated: this endpoint is unbounded and its cost grows with the
    /// owner's collection size. Prefer `get_owner_tokens_page`.
    pub fn get_owner_tokens(env: &Env, owner: Address) -> Vec<u32> {
        extend_instance_ttl(env);
        let key = DataKey::OwnerTokens(owner);
        let tokens = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| vec![&env]);
        extend_persistent_ttl(env, &key);
        tokens
    }

    /// Returns a page of token IDs owned by `owner`.
    ///
    /// Ordering is stable ascending by token ID. `cursor` is the last token ID
    /// returned by the previous page (exclusive lower bound). Returns the page
    /// and the next cursor, or `None` when the page is final.
    pub fn get_owner_tokens_page(
        env: &Env,
        owner: Address,
        cursor: Option<u32>,
        limit: u32,
    ) -> (Vec<u32>, Option<u32>) {
        extend_instance_ttl(env);
        if limit == 0 || limit > MAX_OWNER_TOKENS_PAGE {
            panic_with_error!(env, NonFungibleTokenContractError::InvalidPageLimit);
        }

        let key = DataKey::OwnerTokens(owner);
        let tokens: Vec<u32> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| vec![&env]);
        extend_persistent_ttl(env, &key);

        if let Some(c) = cursor {
            if tokens.first_index_of(c).is_none() {
                panic_with_error!(env, NonFungibleTokenContractError::InvalidCursor);
            }
        }

        let mut page: Vec<u32> = vec![env];
        let mut last: Option<u32> = None;
        let mut has_more = false;
        for token_id in tokens.iter() {
            if let Some(c) = cursor {
                if token_id <= c {
                    continue;
                }
            }
            if page.len() >= limit {
                has_more = true;
                break;
            }
            page.push_back(token_id);
            last = Some(token_id);
        }

        let next_cursor = if has_more { last } else { None };
        (page, next_cursor)
    }

    pub fn mint_with_data(env: &Env, to: Address, token_id: u32, data: TokenData) -> u32 {
        extend_instance_ttl(env);
        let token_id = Self::mint(env, to.clone(), token_id);
        Self::set_token_data(env, token_id, data);
        token_id
    }

    pub fn mint(env: &Env, to: Address, token_id: u32) -> u32 {
        extend_instance_ttl(env);
        Self::only_owner(env);
        Self::do_mint(env, &to, token_id);
        token_id
    }

    fn do_mint(env: &Env, to: &Address, token_id: u32) {
        let current_supply = Self::total_supply(env);
        let max_supply = Self::get_max_supply(env);

        if current_supply >= max_supply {
            panic_with_error!(env, NonFungibleTokenContractError::SupplyExhausted);
        }
        if env
            .storage()
            .persistent()
            .has(&NFTStorageKey::Owner(token_id))
        {
            panic_with_error!(env, NonFungibleTokenContractError::AlreadyMinted);
        }
        increase_total_minted(env);
        add_token_to_owner_list(env, to, token_id);
        Base::mint(env, to, token_id);
        extend_nft_persistent_ttl(env, &NFTStorageKey::Owner(token_id));
        extend_nft_persistent_ttl(env, &NFTStorageKey::Balance(to.clone()));
    }

    pub fn get_token_metadata(env: &Env) -> TokenMetadata {
        extend_instance_ttl(env);
        let metadata = Base::get_metadata(env);

        if metadata.base_uri.is_empty() {
            panic_with_error!(env, NonFungibleTokenError::UnsetMetadata);
        }

        TokenMetadata {
            name: metadata.name,
            symbol: metadata.symbol,
            base_uri: metadata.base_uri,
        }
    }

    pub fn bulk_mint_with_data(env: &Env, tokens: Vec<(Address, u32, TokenData)>) {
        extend_instance_ttl(env);
        Self::only_owner(env);

        let count = tokens.len();
        if count > MAX_BULK_MINT {
            panic_with_error!(env, NonFungibleTokenContractError::MaxBatchExceeded);
        }

        let current_supply = Self::total_supply(env);
        let max_supply = Self::get_max_supply(env);
        if current_supply.saturating_add(count) > max_supply {
            panic_with_error!(env, NonFungibleTokenContractError::InsufficientSupply);
        }

        for (to, token_id, data) in tokens.iter() {
            Self::do_mint(env, &to, token_id);
            Self::set_token_data(env, token_id, data);
        }
    }
}

#[contractimpl]
impl NonFungibleToken for Contract {
    type ContractType = Base;
    fn balance(e: &Env, owner: Address) -> u32 {
        Base::balance(e, &owner)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        Base::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        extend_instance_ttl(e);
        Base::transfer(e, &from, &to, token_id);
        remove_token_from_owner_list(e, &from, token_id);
        add_token_to_owner_list(e, &to, token_id);
        extend_nft_persistent_ttl(e, &NFTStorageKey::Owner(token_id));
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        extend_instance_ttl(e);
        Base::transfer_from(e, &spender, &from, &to, token_id);
        remove_token_from_owner_list(e, &from, token_id);
        add_token_to_owner_list(e, &to, token_id);
        extend_nft_persistent_ttl(e, &NFTStorageKey::Owner(token_id));
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        extend_instance_ttl(e);
        Base::approve(e, &approver, &approved, token_id, live_until_ledger);
        extend_nft_persistent_ttl(e, &NFTStorageKey::Approve(token_id));
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        extend_instance_ttl(e);
        Base::approve_for_all(e, &owner, &operator, live_until_ledger);
        extend_nft_persistent_ttl(e, &NFTStorageKey::Approval(owner, operator));
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        Base::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Base::is_approved_for_all(e, &owner, &operator)
    }

    fn name(e: &Env) -> String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol(e)
    }

    fn token_uri(e: &Env, token_id: u32) -> String {
        Base::token_uri(e, token_id)
    }
}

#[contractimpl]
impl NonFungibleBurnable for Contract {
    fn burn(e: &Env, from: Address, token_id: u32) {
        extend_instance_ttl(e);
        Base::burn(e, &from, token_id);
        remove_token_from_owner_list(e, &from, token_id);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        extend_instance_ttl(e);
        Base::burn_from(e, &spender, &from, token_id);
        remove_token_from_owner_list(e, &from, token_id);
    }
}

fn add_token_to_owner_list(env: &Env, owner: &Address, token_id: u32) {
    let key = DataKey::OwnerTokens(owner.clone());
    let mut tokens: Vec<u32> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| vec![env]);
    tokens.push_back(token_id);
    env.storage().persistent().set(&key, &tokens);
    extend_persistent_ttl(env, &key);
}

fn remove_token_from_owner_list(env: &Env, owner: &Address, token_id: u32) {
    let key = DataKey::OwnerTokens(owner.clone());
    let mut tokens: Vec<u32> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| {
            panic_with_error!(env, NonFungibleTokenContractError::TokenDoesNotExist)
        });
    if let Some(pos) = tokens.first_index_of(token_id) {
        tokens.remove(pos);
        env.storage().persistent().set(&key, &tokens);
    }
    extend_persistent_ttl(env, &key);
}

fn increase_total_minted(env: &Env) {
    // Unwrap is safe because TotalMinted is set in constructor
    let mut current_minted: u32 = env.storage().instance().get(&DataKey::TotalMinted).unwrap();
    current_minted += 1;
    env.storage()
        .instance()
        .set(&DataKey::TotalMinted, &current_minted);
}
