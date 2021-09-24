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

    #[error("Cannot liquidate safely collateralized loan")]
    CannotLiquidateSafeLoan {},

    #[error("An epoch has not passed yet; last executed height: {0}")]
    EpochNotPassed(u64),

    #[error("Token is already registered as collateral")]
    TokenAlreadyRegistered {},

    #[error("Unlock amount cannot exceed locked amount")]
    UnlockExceedsLocked {},

    #[error("Unlock amount too high; Loan liability becomes greater than borrow limit: {0}")]
    UnlockTooLarge(u128),
}
