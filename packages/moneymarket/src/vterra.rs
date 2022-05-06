use cosmwasm_bignumber::{Decimal256, Uint256};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Owner address for config update
    pub owner_addr: String,
    /// Anchor ve token code ID used to instantiate
    pub vterra_code_id: u64,
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
    pub min_gross_rate: Decimal256
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
        vterra_contract: Option<String>,

        max_pos_change: Option<Decimal256>,
        max_neg_change: Option<Decimal256>,
        max_rate: Option<Decimal256>,
        min_rate: Option<Decimal256>,
        diff_multiplier: Option<Decimal256>,
    },

    ////////////////////
    /// Overseer operations
    ////////////////////

    /// Update premium rate
    ExecuteEpochOperations {
        target_aterra_deposit_rate: Decimal256,
    },

    ////////////////////
    /// User operations
    ////////////////////

    /// Rebond `amount` of locked aterra back into vterra without having to wait full 30 days
    RebondLockedATerra {
        amount: Option<Uint256>,
    },

    /// Claim `amount` of aterra unbonded 30 days before `block_height`
    ClaimATerra {
        amount: Option<Uint256>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Bond aterra and release vterra
    BondATerra {},

    /// Burn vterra and entitle sender to claim corresponding aterra after 30 day waiting period
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
    pub vterra_contract: String,

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
    /// Cached vterra supply.
    /// This is kept locally to not require expensive queries to CW20 contract
    pub vterra_supply: Uint256,
    /// Exchange rate between vterra and aterra calculated during last ExecuteEpochOperations
    pub prev_epoch_vterra_exchange_rate: Decimal256,
    /// Target share of deposits in vterra. o
    /// Premium rate adjusts to bring current share towards target share
    pub target_share: Decimal256,
    /// Current premium rate of vterra over aterra measured in blocks
    /// ex. 2% yearly premium => 1.02 / num_blocks_per_year
    pub premium_rate: Decimal256, // in blocks
    /// Block height ExecuteEpochOperations was last executed on
    pub last_updated: u64,
}

/// Exchange rate of aterra / vterra
/// ex: 1 ve * ER => ER aterra
pub fn compute_ve_exchange_rate(
    previous_er: Decimal256,
    premium_rate: Decimal256,
    last_updated: u64,
    block_height: u64,
) -> Decimal256 {
    let blocks_elapsed = block_height - last_updated;
    if blocks_elapsed == 0 {
        previous_er
    } else {
        previous_er * pow(premium_rate, blocks_elapsed)
    }
}

pub fn pow(base: Decimal256, power: u64) -> Decimal256 {
    let mut acc = Decimal256::one();
    for _ in 0..power {
        acc = acc * base
    }
    acc
}
