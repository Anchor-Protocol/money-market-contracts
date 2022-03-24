use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::tokens::TokensHuman;
use cosmwasm_bignumber::{Decimal256, Uint256};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Initial owner address
    pub owner_addr: String,
    /// Oracle contract address for collateral tokens
    pub oracle_contract: String,
    /// Market contract address to receive missing interest buffer
    pub market_contract: String,
    /// Liquidation model contract address to compute liquidation amount
    pub liquidation_contract: String,
    /// Collector contract address which is purchasing ANC token
    pub collector_contract: String,
    /// The base denomination used when fetching oracle price,
    /// reward distribution, and borrow
    pub stable_denom: String,
    /// # of blocks per epoch period
    pub epoch_period: u64,
    /// Distribute interest buffer to market contract,
    /// when deposit_rate < threshold_deposit_rate
    pub threshold_deposit_rate: Decimal256,
    /// Target deposit rate.
    /// When current deposit rate is bigger than this,
    /// Custody contracts send rewards to interest buffer
    pub target_deposit_rate: Decimal256,
    /// Ratio to be distributed from the interest buffer
    pub buffer_distribution_factor: Decimal256,
    /// Ratio to be used for purchasing ANC token from the interest buffer
    pub anc_purchase_factor: Decimal256,
    /// Valid oracle price timeframe
    pub price_timeframe: u64,
    /// # of blocks per each dynamic rate change period
    pub dyn_rate_epoch: u64,
    /// maximum rate change during update
    pub dyn_rate_maxchange: Decimal256,
    /// amount of slack in yr change to trigger rate update
    pub dyn_rate_yr_increase_expectation: Decimal256,
    /// clamps for dyn rate
    pub dyn_rate_min: Decimal256,
    pub dyn_rate_max: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    /// Period of time in blocks when rate is evaluated/changed
    pub dyn_rate_epoch: u64,
    /// Maximum allowed rate change per epoch
    pub dyn_rate_maxchange: Decimal256,
    /// Margin to define expectation of rate increase
    pub dyn_rate_yr_increase_expectation: Decimal256,

    pub dyn_rate_min: Decimal256,
    pub dyn_rate_max: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    ////////////////////
    /// Owner operations
    ////////////////////

    /// Update Configs
    UpdateConfig {
        owner_addr: Option<String>,
        oracle_contract: Option<String>,
        liquidation_contract: Option<String>,
        threshold_deposit_rate: Option<Decimal256>,
        target_deposit_rate: Option<Decimal256>,
        buffer_distribution_factor: Option<Decimal256>,
        anc_purchase_factor: Option<Decimal256>,
        epoch_period: Option<u64>,
        price_timeframe: Option<u64>,
        dyn_rate_epoch: Option<u64>,
        dyn_rate_maxchange: Option<Decimal256>,
        dyn_rate_yr_increase_expectation: Option<Decimal256>,
        dyn_rate_min: Option<Decimal256>,
        dyn_rate_max: Option<Decimal256>,
    },
    /// Create new custody contract for the given collateral token
    Whitelist {
        name: String,             // bAsset name
        symbol: String,           // bAsset symbol
        collateral_token: String, // bAsset token contract
        custody_contract: String, // bAsset custody contract
        max_ltv: Decimal256,      // Loan To Value ratio
    },
    /// Update registered whitelist info
    UpdateWhitelist {
        collateral_token: String,         // bAsset token contract
        custody_contract: Option<String>, // bAsset custody contract
        max_ltv: Option<Decimal256>,      // Loan To Value ratio
    },

    /// Claims all staking rewards from the bAsset contracts
    /// and also do a epoch basis updates
    /// 1. Distribute interest buffers to depositors
    /// 2. Invoke [Custody] DistributeRewards
    /// 3. Update epoch state
    ExecuteEpochOperations {},
    UpdateEpochState {
        interest_buffer: Uint256,
        distributed_interest: Uint256,
    },

    ////////////////////
    /// User operations
    ////////////////////
    LockCollateral {
        collaterals: TokensHuman, // <(Collateral Token, Amount)>
    },
    UnlockCollateral {
        collaterals: TokensHuman, // <(Collateral Token, Amount)>
    },

    /////////////////////////////
    /// Permissionless operations
    /////////////////////////////
    LiquidateCollateral {
        borrower: String,
    },

    FundReserve {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    EpochState {},
    DynrateState {},
    Whitelist {
        collateral_token: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    Collaterals {
        borrower: String,
    },
    AllCollaterals {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    BorrowLimit {
        borrower: String,
        block_time: Option<u64>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner_addr: String,
    pub oracle_contract: String,
    pub market_contract: String,
    pub liquidation_contract: String,
    pub collector_contract: String,
    pub threshold_deposit_rate: Decimal256,
    pub target_deposit_rate: Decimal256,
    pub buffer_distribution_factor: Decimal256,
    pub anc_purchase_factor: Decimal256,
    pub stable_denom: String,
    pub epoch_period: u64,
    pub price_timeframe: u64,
    pub dyn_rate_epoch: u64,
    pub dyn_rate_maxchange: Decimal256,
    pub dyn_rate_yr_increase_expectation: Decimal256,
    pub dyn_rate_min: Decimal256,
    pub dyn_rate_max: Decimal256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistResponseElem {
    pub name: String,
    pub symbol: String,
    pub max_ltv: Decimal256,
    pub custody_contract: String,
    pub collateral_token: String,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistResponse {
    pub elems: Vec<WhitelistResponseElem>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollateralsResponse {
    pub borrower: String,
    pub collaterals: TokensHuman, // <(Collateral Token, Amount)>
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllCollateralsResponse {
    pub all_collaterals: Vec<CollateralsResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowLimitResponse {
    pub borrower: String,
    pub borrow_limit: Uint256,
}
