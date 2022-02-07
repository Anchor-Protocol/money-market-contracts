use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::custody::BAssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// owner address
    pub owner: String,
    /// queries prices to keep track of staking
    pub oracle: String,
    /// rebasing token address (address to wrapped token on terra, i.e. wmSOL)
    pub collateral_token: String,
    /// underlying token address (i.e. wSOL)
    pub underlying_token: String,
    /// overseer contract address
    pub overseer_contract: String,
    /// market contract address
    pub market_contract: String,
    /// liquidation contract address
    pub liquidation_contract: String,
    /// Expected reward denom. If bAsset reward is not same with
    /// it, we try to convert the reward to the `stable_denom`.
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub oracle: String,
    pub collateral_token: String,
    pub underlying_token: String,
    pub overseer_contract: String,
    pub market_contract: String,
    pub liquidation_contract: String,
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}
