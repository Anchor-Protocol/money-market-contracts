use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    /// overseer contract can be registered after
    /// we register market contract to overseer contract
    // pub overseer_contract: HumanAddr,

    /// Owner address for config update
    pub owner_addr: HumanAddr,
    /// The contract has the calculation logics
    /// of Anchor borrow interest rate
    pub interest_model: HumanAddr,
    /// stable coin denom used to borrow & repay
    pub base_denom: String,
    /// reserve ratio applied to interest
    pub reserve_factor: Decimal,
    /// Anchor token code ID used to instantiate
    pub anchor_token_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////
    /// Owner operations
    ////////////////////
    /// Register Overseer contract address
    RegisterOverseer {
        overseer_contract: HumanAddr,
    },

    /// (internal) Register Anchor token contract address
    /// Anchor Token will invoke this after init
    RegisterAnchorToken {},

    /// Update config values
    UpdateConfig {
        owner_addr: Option<HumanAddr>,
        reserve_factor: Option<Decimal>,
        interest_model: Option<HumanAddr>,
    },

    ////////////////////
    /// User operations
    ////////////////////
    /// Deposit stable asset to get interest
    DepositStable {},

    /// Borrow stable asset with collaterals in overseer contract
    BorrowStable {
        borrow_amount: Uint128,
        to: Option<HumanAddr>,
    },

    /// Repay stable asset to decrease liability
    RepayStable {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Return stable coins to a user
    /// according to exchange rate
    RedeemStable {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    Liability {
        borrower: HumanAddr,
    },
    Liabilitys {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    LoanAmount {
        borrower: HumanAddr,
        block_height: u64,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner_addr: HumanAddr,
    pub anchor_token: HumanAddr,
    pub interest_model: HumanAddr,
    pub overseer_contract: HumanAddr,
    pub base_denom: String,
    pub reserve_factor: Decimal,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiabilityResponse {
    pub borrower: HumanAddr,
    pub interest_index: Decimal,
    pub loan_amount: Uint128,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiabilitysResponse {
    pub liabilitys: Vec<LiabilityResponse>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoanAmountResponse {
    pub borrower: HumanAddr,
    pub loan_amount: Uint128,
}
