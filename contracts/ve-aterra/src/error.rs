use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{OverflowError, StdError, Timestamp};
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

    #[error("veaTerra unlock block timestamp not yet reached. Current {0}, Required {1}")]
    VeStakeNotUnlocked(Timestamp, Timestamp),

    #[error("Not enough aterra unlocked. Requested {0}, Available {1}")]
    NotEnoughUnlocked(Uint256, Uint256),

    #[error("Repay amount must be greater than 0 {0}")]
    ZeroRepay(String),

    #[error("Provided CW20 hook is unsupported {0}")]
    UnsupportedCw20Hook(String),
}
