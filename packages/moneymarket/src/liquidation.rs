use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cw20::Cw20ReceiveMsg;

use crate::tokens::TokensHuman;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub oracle_contract: String,
    pub stable_denom: String,
    /// borrow_amount / borrow_limit must always be bigger than  
    /// safe_ratio.
    pub safe_ratio: Decimal256,
    /// Fee applied to executed bids
    /// Sent to Overseer interest buffer
    pub bid_fee: Decimal256,
    /// Maximum fee applied to liquidated collaterals
    /// Sent to liquidator as incentive
    pub max_premium_rate: Decimal256,
    /// Liquidation threshold amount in stable denom.
    /// When the current collaterals value is smaller than
    /// the threshold, all collaterals will be liquidated
    pub liquidation_threshold: Uint256,
    /// Valid oracle price timeframe
    pub price_timeframe: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateConfig {
        owner: Option<String>,
        oracle_contract: Option<String>,
        stable_denom: Option<String>,
        safe_ratio: Option<Decimal256>,
        bid_fee: Option<Decimal256>,
        max_premium_rate: Option<Decimal256>,
        liquidation_threshold: Option<Uint256>,
        price_timeframe: Option<u64>,
    },
    SubmitBid {
        collateral_token: String,
        premium_rate: Decimal256,
    },
    RetractBid {
        collateral_token: String,
        amount: Option<Uint256>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    ExecuteBid {
        liquidator: String,
        fee_address: Option<String>,
        repay_address: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    LiquidationAmount {
        borrow_amount: Uint256,
        borrow_limit: Uint256,
        collaterals: TokensHuman,
        collateral_prices: Vec<Decimal256>,
    },
    Bid {
        collateral_token: String,
        bidder: String,
    },
    BidsByUser {
        bidder: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    BidsByCollateral {
        collateral_token: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub oracle_contract: String,
    pub stable_denom: String,
    pub safe_ratio: Decimal256,
    pub bid_fee: Decimal256,
    pub max_premium_rate: Decimal256,
    pub liquidation_threshold: Uint256,
    pub price_timeframe: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationAmountResponse {
    pub collaterals: TokensHuman,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidResponse {
    pub collateral_token: String,
    pub bidder: String,
    pub amount: Uint256,
    pub premium_rate: Decimal256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidsResponse {
    pub bids: Vec<BidResponse>,
}
