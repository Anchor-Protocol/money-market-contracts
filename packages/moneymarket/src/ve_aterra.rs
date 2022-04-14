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
    /// Address of aterra (aUST) cw20 contract
    pub aterra_contract: String,
    pub stable_denom: String,

    pub target_share: Decimal256,
    pub max_pos_change: Decimal256,
    pub max_neg_change: Decimal256,
    pub max_rate: Decimal256,
    pub min_rate: Decimal256,
    pub diff_multiplier: Decimal256,
    /// percentage points target_share moves towards end_goal_share per epoch
    pub target_transition_amount: Decimal256,
    pub premium_rate: Decimal256,
    pub target_transition_epoch: u64,
    /// End goal for percent of total deposits held as ve_aterra compared to aterra
    pub end_goal_ve_share: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////
    /// Owner operations
    ////////////////////
    /// Update config values
    UpdateConfig {},

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
    pub aterra_contract: String,
    pub ve_aterra_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateResponse {
    // todo
}
