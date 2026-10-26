use soroban_sdk::contracterror;

#[contracterror]
pub enum NonFungibleTokenContractError {
    SupplyExhausted = 1,
    UnsetMaxSupply = 2,
    UnsetTotalMinted = 3,
    UnsetOwner = 4,
    UnsetTokenData = 5,
    AlreadyMinted = 6,
    TokenDoesNotExist = 7,
    MaxBatchExceeded = 8,
    InsufficientSupply = 9,
    InvalidCursor = 10,
}