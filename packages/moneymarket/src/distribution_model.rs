use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Decimal256;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub emission_cap: Decimal256,
    pub emission_floor: Decimal256,
    pub increment_multiplier: Decimal256,
    pub decrement_multiplier: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        owner: Option<String>,
        emission_cap: Option<Decimal256>,
        emission_floor: Option<Decimal256>,
        increment_multiplier: Option<Decimal256>,
        decrement_multiplier: Option<Decimal256>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    AncEmissionRate {
        deposit_rate: Decimal256,
        target_deposit_rate: Decimal256,
        threshold_deposit_rate: Decimal256,
        current_emission_rate: Decimal256,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub emission_cap: Decimal256,
    pub emission_floor: Decimal256,
    pub increment_multiplier: Decimal256,
    pub decrement_multiplier: Decimal256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AncEmissionRateResponse {
    pub emission_rate: Decimal256,
}
