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

    #[error("Borrow amount too high; Loan liability becomes greater than borrow limit: {0}")]
    BorrowExceedsLimit(u128),

    #[error("Must deposit initial funds {0}{0}")]
    InitialFundsNotDeposited(u128, String),

    #[error("Invalid reply ID")]
    InvalidReplyId {},

    #[error("Exceeds {0} max borrow factor; borrow demand too high")]
    MaxBorrowFactorReached(String),

    #[error("Invalid request: \"redeem stable\" message not included in request")]
    MissingRedeemStableHook {},

    #[error("Not enough {0} available; borrow demand too high")]
    NoStableAvailable(String),

    #[error("No staker unlock at given block height")]
    NoUnlockMatchingBlockHeight,

    #[error("veaTerra unlock block timestamp not yet reached. Current {0}, Required {1}")]
    VeStakeNotUnlocked(Timestamp, Timestamp),

    #[error("Not enough aterra unlocked. Requested {0}, Available {1}")]
    NotEnoughUnlocked(Uint256, Uint256),

    #[error("Deposit amount must be greater than 0 {0}")]
    ZeroDeposit(String),

    #[error("Repay amount must be greater than 0 {0}")]
    ZeroRepay(String),
}
