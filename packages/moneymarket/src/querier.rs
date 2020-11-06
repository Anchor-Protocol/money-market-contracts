use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    from_binary, to_binary, AllBalanceResponse, Api, BalanceResponse, BankQuery, Binary, Coin,
    Decimal, Extern, HumanAddr, Querier, QueryRequest, StdError, StdResult, Storage, Uint128,
    WasmQuery,
};

use cosmwasm_storage::to_length_prefixed;
use cw20::TokenInfoResponse;
use terra_cosmwasm::TerraQuerier;

pub fn load_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account_addr: &HumanAddr,
    denom: String,
) -> StdResult<Uint128> {
    // load price form the oracle
    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: HumanAddr::from(account_addr),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

pub fn load_all_balances<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account_addr: &HumanAddr,
) -> StdResult<Vec<Coin>> {
    // load price form the oracle
    let balances: AllBalanceResponse =
        deps.querier
            .query(&QueryRequest::Bank(BankQuery::AllBalances {
                address: HumanAddr::from(account_addr),
            }))?;
    Ok(balances.amount)
}

pub fn load_supply<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_addr: &HumanAddr,
) -> StdResult<Uint128> {
    // load price form the oracle
    let res: Binary = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: HumanAddr::from(token_addr),
        key: Binary::from(to_length_prefixed(b"token_info")),
    }))?;

    let token_info: TokenInfoResponse = from_binary(&res)?;
    Ok(token_info.total_supply)
}

pub fn load_token_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_addr: &HumanAddr,
    account_addr: &HumanAddr,
) -> StdResult<Uint128> {
    // load balance form the token contract
    let res: Binary = deps
        .querier
        .query(&QueryRequest::Wasm(WasmQuery::Raw {
            contract_addr: HumanAddr::from(token_addr),
            key: Binary::from(concat(
                &to_length_prefixed(b"balance").to_vec(),
                (deps.api.canonical_address(&account_addr)?).as_slice(),
            )),
        }))
        .unwrap_or_else(|_| to_binary(&Uint128::zero()).unwrap());

    from_binary(&res)
}

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
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionParamsResponse {
    pub deposit_rate: Decimal,
    pub target_deposit_rate: Decimal,
}

pub fn load_distribution_params<S: Storage, A: Api, Q: Querier>(
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

pub fn load_epoch_state<S: Storage, A: Api, Q: Querier>(
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
pub fn load_loan_amount<S: Storage, A: Api, Q: Querier>(
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

pub fn load_price<S: Storage, A: Api, Q: Querier>(
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

pub fn load_borrow_rate<S: Storage, A: Api, Q: Querier>(
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

pub fn load_borrow_limit<S: Storage, A: Api, Q: Querier>(
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

#[inline]
fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut k = namespace.to_vec();
    k.extend_from_slice(key);
    k
}
