use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{to_binary, Addr, Deps, QueryRequest, StdResult, WasmQuery};

use moneymarket::liquidation::{LiquidationAmountResponse, QueryMsg as LiquidationQueryMsg};
use moneymarket::market::{
    BorrowerInfoResponse, EpochStateResponse, QueryMsg as MarketQueryMsg, StateResponse,
};
use moneymarket::tokens::TokensHuman;

pub fn query_market_state(
    deps: Deps,
    market_addr: Addr,
    block_height: u64,
) -> StdResult<StateResponse> {
    let epoch_state: StateResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: market_addr.to_string(),
        msg: to_binary(&MarketQueryMsg::State {
            block_height: Some(block_height),
        })?,
    }))?;

    Ok(epoch_state)
}

pub fn query_epoch_state(
    deps: Deps,
    market_addr: Addr,
    block_height: u64,
    distributed_interest: Option<Uint256>,
) -> StdResult<EpochStateResponse> {
    let epoch_state: EpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: market_addr.to_string(),
            msg: to_binary(&MarketQueryMsg::EpochState {
                block_height: Some(block_height),
                distributed_interest,
            })?,
        }))?;

    Ok(epoch_state)
}

/// Query borrow amount from the market contract
pub fn query_borrower_info(
    deps: Deps,
    market_addr: Addr,
    borrower: Addr,
    block_height: u64,
) -> StdResult<BorrowerInfoResponse> {
    let borrower_amount: BorrowerInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: market_addr.to_string(),
            msg: to_binary(&MarketQueryMsg::BorrowerInfo {
                borrower: borrower.to_string(),
                block_height: Some(block_height),
            })?,
        }))?;

    Ok(borrower_amount)
}

#[allow(clippy::ptr_arg)]
pub fn query_liquidation_amount(
    deps: Deps,
    liquidation_contract: Addr,
    borrow_amount: Uint256,
    borrow_limit: Uint256,
    collaterals: &TokensHuman,
    collateral_prices: Vec<Decimal256>,
) -> StdResult<LiquidationAmountResponse> {
    let liquidation_amount_res: LiquidationAmountResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: liquidation_contract.to_string(),
            msg: to_binary(&LiquidationQueryMsg::LiquidationAmount {
                borrow_amount,
                borrow_limit,
                collaterals: collaterals.clone(),
                collateral_prices,
            })?,
        }))?;

    Ok(liquidation_amount_res)
}
