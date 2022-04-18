use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid reply ID")]
    InvalidReplyId {},

    #[error("No staker unlock at given block height")]
    NoUnlockMatchingBlockHeight,

    #[error("Not enough aterra unlocked. Requested {0}, Available {1}")]
    NotEnoughUnlocked(Uint256, Uint256),

    #[error("Provided CW20 hook is unsupported {0}")]
    UnsupportedCw20Hook(String),

    #[error("Cannot execute epoch operations yet, epoch has not passed. Last updated: {0}")]
    EpochNotPassed(u64),
}
