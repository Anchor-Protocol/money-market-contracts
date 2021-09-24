use cosmwasm_std::{Addr, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("User already has bid for specified collateral: {0}")]
    AlreadyBidForCollateral(Addr),

    #[error("No {0} assets have been provided")]
    AssetNotProvided(String),

    #[error("Premium rate cannot exceed the max premium rate: {0}")]
    PremiumExceedsMaxPremium(String),

    #[error("Invalid request: \"execute bid\" message not included in request")]
    MissingExecuteBidHook {},

    #[error("No bids with the specified information exist")]
    NoBidExists {},

    #[error("Insufficient bid balance; Required balance: {0}")]
    InsufficientBidBalance(u128),

    #[error("Retract amount cannot exceed bid balance: {0}")]
    RetractExceedsBid(u128),
}
