use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Api, Extern, HumanAddr, Querier, QueryRequest, StdResult, Storage, WasmQuery,
};

use moneymarket::distribution_model::{AncEmissionRateResponse, QueryMsg as DistributionQueryMsg};
use moneymarket::interest_model::{BorrowRateResponse, QueryMsg as InterestQueryMsg};
use moneymarket::overseer::{BorrowLimitResponse, QueryMsg as OverseerQueryMsg};

pub fn query_borrow_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    interest_addr: &HumanAddr,
    market_balance: Uint256,
    total_liabilities: Decimal256,
    total_reserves: Decimal256,
) -> StdResult<BorrowRateResponse> {
    let borrow_rate: BorrowRateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(interest_addr),
            msg: to_binary(&InterestQueryMsg::BorrowRate {
                market_balance,
                total_liabilities,
                total_reserves,
            })?,
        }))?;

    Ok(borrow_rate)
}

pub fn query_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    overseer_addr: &HumanAddr,
    borrower: &HumanAddr,
    block_time: Option<u64>,
) -> StdResult<BorrowLimitResponse> {
    let borrow_limit: BorrowLimitResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(overseer_addr),
            msg: to_binary(&OverseerQueryMsg::BorrowLimit {
                borrower: HumanAddr::from(borrower),
                block_time,
            })?,
        }))?;

    Ok(borrow_limit)
}

pub fn query_anc_emission_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    distribution_model: &HumanAddr,
    target_deposit_rate: Decimal256,
    deposit_rate: Decimal256,
    current_emission_rate: Decimal256,
) -> StdResult<AncEmissionRateResponse> {
    let anc_emission_rate: AncEmissionRateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(distribution_model),
            msg: to_binary(&DistributionQueryMsg::AncEmissionRate {
                target_deposit_rate,
                deposit_rate,
                current_emission_rate,
            })?,
        }))?;

    Ok(anc_emission_rate)
}
