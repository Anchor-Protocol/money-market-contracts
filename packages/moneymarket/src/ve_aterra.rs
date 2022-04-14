use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::Timestamp;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Owner address for config update
    pub owner_addr: String,
    /// Anchor ve token code ID used to instantiate
    pub ve_aterra_code_id: u64,
    /// Address of market contract
    pub market_addr: String,
    /// Address of overseer contract
    pub overseer_addr: String,
    /// Address of aterra (aUST) cw20 contract
    pub aterra_contract: String,
    pub stable_denom: String,

    pub initial_premium_rate: Decimal256,
    pub premium_rate_epoch: u64,
    pub target_share: Decimal256,
    pub max_pos_change: Decimal256,
    pub max_neg_change: Decimal256,
    pub max_rate: Decimal256,
    pub min_rate: Decimal256,
    pub diff_multiplier: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////
    /// Owner operations
    ////////////////////
    /// Update config values
    UpdateConfig {
        owner_addr: Option<String>,
        market_addr: Option<String>,
        aterra_contract: Option<String>,
        ve_aterra_contract: Option<String>,

        max_pos_change: Option<Decimal256>,
        max_neg_change: Option<Decimal256>,
        max_rate: Option<Decimal256>,
        min_rate: Option<Decimal256>,
        diff_multiplier: Option<Decimal256>,
    },

    ////////////////////
    /// Overseer operations
    ////////////////////
    ExecuteEpochOperations {},

    ////////////////////
    /// User operations
    ////////////////////
    /// Claim `amount` of aterra unbonded 30 days before `block_height`
    ClaimATerra {
        amount: Uint256,
        unlock_time: Timestamp,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Bond aterra and release ve_aterra
    BondATerra {},

    /// Burn ve_aterra and entitle sender to claim corresponding aterra after 30 day waiting period
    UnbondVeATerra {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State { block_height: Option<u64> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub contract_addr: String,
    pub owner_addr: String,
    pub market_addr: String,
    pub overseer_addr: String,
    pub aterra_contract: String,
    pub ve_aterra_contract: String,

    pub max_pos_change: Decimal256,
    pub max_neg_change: Decimal256,
    pub max_rate: Decimal256,
    pub min_rate: Decimal256,
    pub diff_multiplier: Decimal256,

    pub premium_rate_epoch: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateResponse {
    // todo
}

pub fn compute_ve_exchange_rate(
    previous_er: Decimal256,
    premium_rate: Decimal256,
    last_updated: u64,
    block_height: u64,
) -> Decimal256 {
    let blocks_elapsed = Decimal256::from_ratio(block_height - last_updated, 1);
    if blocks_elapsed.is_zero() {
        previous_er
    } else {
        previous_er * blocks_elapsed * premium_rate
    }
}