use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Api, Coin, Decimal, Extern, HumanAddr, Querier, QueryRequest, StdError, StdResult,
    Storage, Uint128, WasmQuery,
};

use terra_cosmwasm::TerraQuerier;

use crate::tokens::TokensHuman;

static DECIMAL_FRACTION: Uint128 = Uint128(1_000_000_000_000_000_000u128);

pub fn compute_tax<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    coin: &Coin,
) -> StdResult<Uint128> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate: Decimal = (terra_querier.query_tax_rate()?).rate;
    let tax_cap: Uint128 = (terra_querier.query_tax_cap(coin.denom.to_string())?).cap;
    Ok(std::cmp::min(
        (coin.amount
            - coin.amount.multiply_ratio(
                DECIMAL_FRACTION,
                DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION,
            ))?,
        tax_cap,
    ))
}

pub fn deduct_tax<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    coin: Coin,
) -> StdResult<Coin> {
    let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: (coin.amount - tax_amount)?,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query distribution params to overseer contract
    DistributionParams {},
    /// Query epoch state to market contract
    EpochState {},
    /// Query borrow amount to market contract
    LoanAmount {
        borrower: HumanAddr,
        block_height: u64,
    },
    /// Query oracle price to oracle contract
    Price { base: String, quote: String },
    /// Query borrow rate to interest model contract
    BorrowRate {},
    /// Query borrow limit to overseer contract
    BorrowLimit { borrower: HumanAddr },
    /// Query liquidation amount to liquidation model contract
    LiquidationAmount {
        borrow_amount: Uint128,
        borrow_limit: Uint128,
        stable_denom: String,
        collaterals: TokensHuman,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionParamsResponse {
    pub deposit_rate: Decimal,
    pub target_deposit_rate: Decimal,
}

pub fn query_distribution_params<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    overseer_addr: &HumanAddr,
) -> StdResult<DistributionParamsResponse> {
    let distribution_params: DistributionParamsResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(overseer_addr),
            msg: to_binary(&QueryMsg::DistributionParams {})?,
        }))?;

    if distribution_params.deposit_rate > Decimal::one() {
        return Err(StdError::generic_err(format!(
            "Invalid deposit_rate {:?}",
            distribution_params.deposit_rate
        )));
    }

    if distribution_params.target_deposit_rate > Decimal::one() {
        return Err(StdError::generic_err(format!(
            "Invalid target_deposit_rate {:?}",
            distribution_params.target_deposit_rate
        )));
    }

    Ok(distribution_params)
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EpochStateResponse {
    pub exchange_rate: Decimal,
    pub a_token_supply: Uint128,
}

pub fn query_epoch_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    market_addr: &HumanAddr,
) -> StdResult<EpochStateResponse> {
    let epoch_state: EpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(market_addr),
            msg: to_binary(&QueryMsg::EpochState {})?,
        }))?;

    Ok(epoch_state)
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoanAmountResponse {
    pub borrower: HumanAddr,
    pub loan_amount: Uint128,
}

/// Query borrow amount from the market contract
pub fn query_loan_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    market_addr: &HumanAddr,
    borrower: &HumanAddr,
    block_height: u64,
) -> StdResult<LoanAmountResponse> {
    let borrower_amount: LoanAmountResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(market_addr),
            msg: to_binary(&QueryMsg::LoanAmount {
                borrower: HumanAddr::from(borrower),
                block_height,
            })?,
        }))?;

    Ok(borrower_amount)
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub rate: Decimal,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

pub fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle_addr: &HumanAddr,
    base: String,
    quote: String,
) -> StdResult<PriceResponse> {
    let oracle_price: PriceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(oracle_addr),
            msg: to_binary(&QueryMsg::Price { base, quote })?,
        }))?;

    Ok(oracle_price)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowRateResponse {
    pub rate: Decimal,
}

pub fn query_borrow_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    interest_model: &HumanAddr,
) -> StdResult<BorrowRateResponse> {
    let borrow_rate: BorrowRateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(interest_model),
            msg: to_binary(&QueryMsg::BorrowRate {})?,
        }))?;

    Ok(borrow_rate)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowLimitResponse {
    pub borrower: HumanAddr,
    pub borrow_limit: Uint128,
}

pub fn query_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    overseer_addr: &HumanAddr,
    borrower: &HumanAddr,
) -> StdResult<BorrowLimitResponse> {
    let borrow_limit: BorrowLimitResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(overseer_addr),
            msg: to_binary(&QueryMsg::BorrowLimit {
                borrower: HumanAddr::from(borrower),
            })?,
        }))?;

    Ok(borrow_limit)
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationAmountResponse {
    pub collaterals: TokensHuman,
}

#[allow(clippy::ptr_arg)]
pub fn query_liquidation_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    liquidation_model: &HumanAddr,
    borrow_amount: Uint128,
    borrow_limit: Uint128,
    stable_denom: String,
    collaterals: &TokensHuman,
) -> StdResult<LiquidationAmountResponse> {
    let liquidation_amount_res: LiquidationAmountResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(liquidation_model),
            msg: to_binary(&QueryMsg::LiquidationAmount {
                borrow_amount,
                borrow_limit,
                stable_denom,
                collaterals: collaterals.clone(),
            })?,
        }))?;

    Ok(liquidation_amount_res)
}
