use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Api, Extern, HumanAddr, Querier, QueryRequest, StdResult, Storage, WasmQuery,
};

use moneymarket::interest::{BorrowRateResponse, QueryMsg as InterestQueryMsg};
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
