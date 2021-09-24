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

    #[error("Liquidation amount cannot exceed locked amount: {0}")]
    LiquidationAmountExceedsLocked(u128),

    #[error("Lock amount cannot excceed the user's spendable amount: {0}")]
    LockAmountExceedsSpendable(u128),

    #[error("Invalid reply ID")]
    InvalidReplyId {},

    #[error("Invalid request: \"deposit collateral\" message not included in request")]
    MissingDepositCollateralHook {},

    #[error("Unlock amount cannot exceed locked amount: {0}")]
    UnlockAmountExceedsLocked(u128),

    #[error("Withdraw amount cannot exceed the user's spendable amount: {0}")]
    WithdrawAmountExceedsSpendable(u128),
}
