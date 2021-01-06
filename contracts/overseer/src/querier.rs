use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Api, Extern, HumanAddr, Querier, QueryRequest, StdResult, Storage, WasmQuery,
};

use moneymarket::liquidation::{LiquidationAmountResponse, QueryMsg as LiquidationQueryMsg};
use moneymarket::market::{EpochStateResponse, LoanAmountResponse, QueryMsg as MarketQueryMsg};
use moneymarket::tokens::TokensHuman;

pub fn query_epoch_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    market_addr: &HumanAddr,
    block_height: u64,
) -> StdResult<EpochStateResponse> {
    let epoch_state: EpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(market_addr),
            msg: to_binary(&MarketQueryMsg::EpochState {
                block_height: Some(block_height),
            })?,
        }))?;

    Ok(epoch_state)
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
            msg: to_binary(&MarketQueryMsg::LoanAmount {
                borrower: HumanAddr::from(borrower),
                block_height,
            })?,
        }))?;

    Ok(borrower_amount)
}

#[allow(clippy::ptr_arg)]
pub fn query_liquidation_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    liquidation_contract: &HumanAddr,
    borrow_amount: Uint256,
    borrow_limit: Uint256,
    collaterals: &TokensHuman,
    collateral_prices: Vec<Decimal256>,
) -> StdResult<LiquidationAmountResponse> {
    let liquidation_amount_res: LiquidationAmountResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(liquidation_contract),
            msg: to_binary(&LiquidationQueryMsg::LiquidationAmount {
                borrow_amount,
                borrow_limit,
                collaterals: collaterals.clone(),
                collateral_prices,
            })?,
        }))?;

    Ok(liquidation_amount_res)
}
