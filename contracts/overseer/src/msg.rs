use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    /// Initial owner address
    pub owner_addr: HumanAddr,
    /// Oracle contract address for collateral tokens
    pub oracle_contract: HumanAddr,
    /// Market contract address to receive missing interest buffer
    pub market_contract: HumanAddr,
    /// The base denomination used when fetching oracle price,
    /// reward distribution, and borrow
    pub base_denom: String,
    /// Distribute interest buffer, if deposit_rate < distribution_threshold
    pub distribution_threshold: Decimal,
    /// Target deposit rate.
    /// When current deposit rate is bigger than this,
    /// Custody contracts send rewards to interest buffer
    pub target_deposit_rate: Decimal,
    /// Ratio to be distributed from the interest buffer
    pub buffer_distribution_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ////////////////////
    /// Owner operations
    ////////////////////

    /// Update Configs
    UpdateConfig {
        owner_addr: Option<HumanAddr>,
        distribution_threshold: Option<Decimal>,
        target_deposit_rate: Option<Decimal>,
        buffer_distribution_rate: Option<Decimal>,
    },

    /// Make specified amount of tokens unspendable
    Whitelist {
        collateral_token: HumanAddr,
        custody_contract: HumanAddr,
        ltv: Decimal,
    },

    // LiquidateCollateral not yet implemented
    /// Claims all staking rewards from the bAsset contracts
    /// and also do a epoch basis updates
    /// 1. Distribute interest buffers to depositors
    /// 2. Invoke [Market] SendKeeperPremium & [Custody] DistributeRewards
    ExecuteEpochOperations {},

    ////////////////////
    /// User operations
    ////////////////////
    LockCollateral {
        collaterals: Vec<(HumanAddr, Uint128)>, // <(Collateral Token, Amount)>
    },
    UnlockCollateral {
        collaterals: Vec<(HumanAddr, Uint128)>, // <(Collateral Token, Amount)>
    },
    Borrow {
        amount: Uint128, // the amount of stable 
    },

    /////////////////////////////
    /// Permissionless operations
    /////////////////////////////
    LiquidiateCollateral { borrower: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    RegisterAnchorToken {
        anchor_token: HumanAddr,
    },
    Whitelist {
        collateral_token: Option<HumanAddr>,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    Loan {
        borrower: HumanAddr,
    },
    Loans {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner_addr: HumanAddr,
    pub oracle_contract: HumanAddr,
    pub oracle_base_denom: String,
    pub deposit_rate_threshold: Decimal,
    pub buffer_distribution_rate: Decimal,
    pub buffer_tax_rate: Decimal,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistResponseItem {
    pub ltv: Decimal,
    pub custody_contract: HumanAddr,
    pub collateral_token: HumanAddr,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistResponse {
    pub items: Vec<WhitelistResponseItem>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoanResponse {
    pub borrower: HumanAddr,
    pub collaterals: Vec<(HumanAddr, Uint128)>, // <(Collateral Token, Amount)>
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoansResponse {
    pub loans: Vec<LoanResponse>,
}
