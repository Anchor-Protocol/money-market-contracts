use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::HumanAddr;
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    /// owner address
    pub owner: HumanAddr,
    /// bAsset token address
    pub collateral_token: HumanAddr,
    /// overseer contract address
    pub overseer_contract: HumanAddr,
    /// market contract address
    pub market_contract: HumanAddr,
    /// bAsset rewrad contract
    pub reward_contract: HumanAddr,
    /// liquidation contract address
    pub liquidation_contract: HumanAddr,
    /// Expected reward denom. If bAsset reward is not same with
    /// it, we try to convert the reward to the `stable_denom`.
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// CW20 token receiver
    Receive(Cw20ReceiveMsg),

    ////////////////////
    /// Overseer operations
    ////////////////////

    /// Update config
    UpdateConfig {
        owner: Option<HumanAddr>,
        liquidation_contract: Option<HumanAddr>,
    },
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

    /// (internal) Send withdrawn rewards to market & overseer
    DistributeHook {},

    /// (internal) Swap all coins to stable_denom
    SwapToStableDenom {},

    /// Liquidate collateral and send liquidated collateral to `to` address
    LiquidateCollateral {
        liquidator: HumanAddr,
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
pub enum Cw20HookMsg {
    /// Deposit collateral token
    DepositCollateral {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Borrower {
        address: HumanAddr,
    },
    Borrowers {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub collateral_token: HumanAddr,
    pub overseer_contract: HumanAddr,
    pub market_contract: HumanAddr,
    pub reward_contract: HumanAddr,
    pub liquidation_contract: HumanAddr,
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerResponse {
    pub borrower: HumanAddr,
    pub balance: Uint256,
    pub spendable: Uint256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowersResponse {
    pub borrowers: Vec<BorrowerResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BAssetInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
