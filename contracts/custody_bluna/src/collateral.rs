use crate::state::{
    read_borrower_info, read_borrowers, read_config, remove_borrower_info, store_borrower_info,
    BorrowerInfo, Config,
};

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, to_binary, Addr, CanonicalAddr, CosmosMsg, Deps, DepsMut, MessageInfo, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use moneymarket::custody::{BorrowerResponse, BorrowersResponse};
use moneymarket::liquidation::Cw20HookMsg as LiquidationCw20HookMsg;
use terra_cosmwasm::TerraMsgWrapper;

/// Deposit new collateral
/// Executor: bAsset token contract
pub fn deposit_collateral(
    deps: DepsMut,
    borrower: Addr,
    amount: Uint256,
) -> StdResult<Response<TerraMsgWrapper>> {
    let borrower_raw = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(deps.storage, &borrower_raw);

    // withdraw rewards to pending rewards
    borrower_info.balance += amount;
    borrower_info.spendable += amount;

    store_borrower_info(deps.storage, &borrower_raw, &borrower_info)?;

    Ok(Response {
        messages: vec![],
        attributes: vec![
            attr("action", "deposit_collateral"),
            attr("borrower", borrower.as_str()),
            attr("amount", amount.to_string()),
        ],
        events: vec![],
        data: None,
    })
}

/// Withdraw spendable collateral or a specified amount of collateral
/// Executor: borrower
pub fn withdraw_collateral(
    deps: DepsMut,
    info: MessageInfo,
    amount: Option<Uint256>,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;

    let borrower = info.sender;
    let borrower_raw = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(deps.storage, &borrower_raw);

    // Check spendable balance
    let amount = amount.unwrap_or(borrower_info.spendable);
    if borrower_info.spendable < amount {
        return Err(StdError::generic_err(format!(
            "Withdraw amount cannot exceed the user's spendable amount: {}",
            borrower_info.spendable
        )));
    }

    // withdraw rewards to pending rewards
    borrower_info.balance = borrower_info.balance - amount;
    borrower_info.spendable = borrower_info.spendable - amount;

    if borrower_info.balance == Uint256::zero() {
        remove_borrower_info(deps.storage, &borrower_raw);
    } else {
        store_borrower_info(deps.storage, &borrower_raw, &borrower_info)?;
    }

    Ok(Response {
        messages: vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&config.collateral_token)?
                .to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: borrower.to_string(),
                amount: amount.into(),
            })?,
        }))],
        attributes: vec![
            attr("action", "withdraw_collateral"),
            attr("borrower", borrower.as_str()),
            attr("amount", amount.to_string()),
        ],
        events: vec![],
        data: None,
    })
}

/// Decrease spendable collateral to lock
/// specified amount of collateral token
/// Executor: overseer
pub fn lock_collateral(
    deps: DepsMut,
    info: MessageInfo,
    borrower: Addr,
    amount: Uint256,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.overseer_contract {
        return Err(StdError::generic_err("unauthorized"));
    }

    let borrower_raw: CanonicalAddr = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(deps.storage, &borrower_raw);
    if amount > borrower_info.spendable {
        return Err(StdError::generic_err(format!(
            "Lock amount cannot excceed the user's spendable amount: {}",
            borrower_info.spendable
        )));
    }

    borrower_info.spendable = borrower_info.spendable - amount;
    store_borrower_info(deps.storage, &borrower_raw, &borrower_info)?;
    Ok(Response {
        messages: vec![],
        attributes: vec![
            attr("action", "lock_collateral"),
            attr("borrower", borrower),
            attr("amount", amount),
        ],
        events: vec![],
        data: None,
    })
}

/// Increase spendable collateral to unlock
/// specified amount of collateral token
/// Executor: overseer
pub fn unlock_collateral(
    deps: DepsMut,
    info: MessageInfo,
    borrower: Addr,
    amount: Uint256,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.overseer_contract {
        return Err(StdError::generic_err("unauthorized"));
    }

    let borrower_raw: CanonicalAddr = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(deps.storage, &borrower_raw);
    let borrowed_amt = borrower_info.balance - borrower_info.spendable;
    if amount > borrowed_amt {
        return Err(StdError::generic_err(format!(
            "Unlock amount cannot exceed locked amount: {}",
            borrowed_amt
        )));
    }

    borrower_info.spendable += amount;
    store_borrower_info(deps.storage, &borrower_raw, &borrower_info)?;

    Ok(Response {
        messages: vec![],
        attributes: vec![
            attr("action", "unlock_collateral"),
            attr("borrower", borrower),
            attr("amount", amount),
        ],
        events: vec![],
        data: None,
    })
}

pub fn liquidate_collateral(
    deps: DepsMut,
    info: MessageInfo,
    liquidator: Addr,
    borrower: Addr,
    amount: Uint256,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.overseer_contract {
        return Err(StdError::generic_err("unauthorized"));
    }

    let borrower_raw: CanonicalAddr = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(deps.storage, &borrower_raw);
    let borrowed_amt = borrower_info.balance - borrower_info.spendable;
    if amount > borrowed_amt {
        return Err(StdError::generic_err(format!(
            "Liquidation amount cannot exceed locked amount: {}",
            borrowed_amt
        )));
    }

    borrower_info.balance = borrower_info.balance - amount;
    store_borrower_info(deps.storage, &borrower_raw, &borrower_info)?;

    Ok(Response {
        messages: vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&config.collateral_token)?
                .to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: deps
                    .api
                    .addr_humanize(&config.liquidation_contract)?
                    .to_string(),
                amount: amount.into(),
                msg: to_binary(&LiquidationCw20HookMsg::ExecuteBid {
                    liquidator: liquidator.to_string(),
                    fee_address: Some(
                        deps.api
                            .addr_humanize(&config.overseer_contract)?
                            .to_string(),
                    ),
                    repay_address: Some(
                        deps.api.addr_humanize(&config.market_contract)?.to_string(),
                    ),
                })?,
            })?,
        }))],
        attributes: vec![
            attr("action", "liquidate_collateral"),
            attr("liquidator", liquidator),
            attr("borrower", borrower),
            attr("amount", amount),
        ],
        events: vec![],
        data: None,
    })
}

pub fn query_borrower(deps: Deps, borrower: Addr) -> StdResult<BorrowerResponse> {
    let borrower_raw = deps.api.addr_canonicalize(borrower.as_str())?;
    let borrower_info: BorrowerInfo = read_borrower_info(deps.storage, &borrower_raw);
    Ok(BorrowerResponse {
        borrower: borrower.to_string(),
        balance: borrower_info.balance,
        spendable: borrower_info.spendable,
    })
}

pub fn query_borrowers(
    deps: Deps,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BorrowersResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.addr_canonicalize(start_after.as_str())?)
    } else {
        None
    };

    let borrowers = read_borrowers(deps, start_after, limit)?;
    Ok(BorrowersResponse { borrowers })
}
