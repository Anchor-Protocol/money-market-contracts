use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    /// bAsset token address
    pub collateral_token: HumanAddr,
    /// overseer contract address
    pub overseer_contract: HumanAddr,
    /// market contract address
    pub market_contract: HumanAddr,
    /// bAsset rewrad contract
    pub reward_contract: HumanAddr,
    /// Expected reward denom. If bAsset reward is not same with
    /// it, we try to convert the reward to the `reward_denom`.
    pub reward_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// CW20 token receiver
    Receive(Cw20ReceiveMsg),

    ////////////////////
    /// Owner operations
    ////////////////////

    /// Make specified amount of tokens unspendable
    LockCollateral {
        borrower: HumanAddr,
        amount: Uint128,
    },
    /// Make specified amount of collateral tokens spendable
    UnlockCollateral {
        borrower: HumanAddr,
        amount: Uint128,
    },
    /// Claim bAsset rewards and distribute claimed rewards
    /// with reward_index update
    DistributeRewards {},
    DistributeHook {
        prev_balance: Uint128,
    },
    /// Swap all coins to reward_denom
    SwapToRewardDenom {},

    ////////////////////
    /// User operations
    ////////////////////

    /// Withdraw spendable collateral token.
    /// If the amount is not given,
    /// return all spendable collateral
    WithdrawCollateral {
        amount: Option<Uint128>,
    },
    ClaimReward {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Deposit collataerl token
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
    pub collateral_token: HumanAddr,
    pub overseer_contract: HumanAddr,
    pub market_contract: HumanAddr,
    pub reward_contract: HumanAddr,
    pub reward_denom: String,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerResponse {
    pub borrower: HumanAddr,
    pub balance: Uint128,
    pub spendable: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowersResponse {
    pub borrowers: Vec<BorrowerResponse>,
}
