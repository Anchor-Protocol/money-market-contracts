use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RewardContractHandleMsg {
    /// Request bAsset reward withdrawal
    ClaimRewards { recipient: Option<HumanAddr> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RewardContractQueryMsg {
    /// Request bAsset reward amount
    AccruedRewards { address: HumanAddr },
}
