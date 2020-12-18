use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::HumanAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustodyHandleMsg {
    ////////////////////
    /// Owner operations
    ////////////////////

    /// Make specified amount of tokens unspendable
    LockCollateral {
        borrower: HumanAddr,
        amount: Uint256,
    },
    /// Make specified amount of collateral tokens spendable
    UnlockCollateral {
        borrower: HumanAddr,
        amount: Uint256,
    },
    /// Claim bAsset rewards and distribute claimed rewards
    /// to market and overseer contracts
    DistributeRewards {},
    /// Liquidate colalteral and send liquidated collateral to `to` address
    LiquidateCollateral {
        borrower: HumanAddr,
        amount: Uint256,
    },

    ////////////////////
    /// User operations
    ////////////////////

    /// Withdraw spendable collateral token.
    /// If the amount is not given,
    /// return all spendable collateral
    WithdrawCollateral { amount: Option<Uint256> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarketHandleMsg {
    ////////////////////
    /// Overseer operations
    ////////////////////
    /// Repay stable with liquidated collaterals
    RepayStableFromLiquidation {
        borrower: HumanAddr,
        prev_balance: Uint256,
    },

    ////////////////////
    /// User operations
    ////////////////////
    /// Deposit stable asset to get interest
    DepositStable {},

    /// Borrow stable asset with collaterals in overseer contract
    BorrowStable {
        borrow_amount: Uint256,
        to: Option<HumanAddr>,
    },

    /// Repay stable asset to decrease liability
    RepayStable {},
}
