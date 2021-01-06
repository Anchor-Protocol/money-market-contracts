use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::HumanAddr;
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
    pub stable_denom: String,
    /// reserve ratio applied to interest
    pub reserve_factor: Decimal256,
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
        reserve_factor: Option<Decimal256>,
        interest_model: Option<HumanAddr>,
    },

    ////////////////////
    /// Overseer operations
    ////////////////////
    /// Repay stable with liquidated collaterals
    RepayStableFromLiquidation {
        borrower: HumanAddr,
        prev_balance: Uint256,
    },

    ////////////////////
    /// User operations
    ////////////////////
    /// Deposit stable asset to get interest
    DepositStable {},

    /// Borrow stable asset with collaterals in overseer contract
    BorrowStable {
        borrow_amount: Uint256,
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
    EpochState {
        block_height: Option<u64>,
    },
    Liability {
        borrower: HumanAddr,
    },
    Liabilities {
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
    pub stable_denom: String,
    pub reserve_factor: Decimal256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EpochStateResponse {
    pub exchange_rate: Decimal256,
    pub a_token_supply: Uint256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiabilityResponse {
    pub borrower: HumanAddr,
    pub interest_index: Decimal256,
    pub loan_amount: Uint256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiabilitiesResponse {
    pub liabilities: Vec<LiabilityResponse>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoanAmountResponse {
    pub borrower: HumanAddr,
    pub loan_amount: Uint256,
}
