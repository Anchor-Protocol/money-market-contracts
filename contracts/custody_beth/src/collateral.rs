use crate::state::{
    read_borrower_info, read_borrowers, read_config, remove_borrower_info, store_borrower_info,
    BorrowerInfo, Config,
};

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, Querier, StdError, StdResult, Storage, WasmMsg,
};
use cw20::Cw20HandleMsg;
use moneymarket::custody::{BorrowerResponse, BorrowersResponse};
use moneymarket::liquidation::Cw20HookMsg as LiquidationCw20HookMsg;
use terra_cosmwasm::TerraMsgWrapper;

/// Deposit new collateral
/// Executor: bAsset token contract
pub fn deposit_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    borrower: HumanAddr,
    amount: Uint256,
) -> HandleResult<TerraMsgWrapper> {
    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);

    borrower_info.balance += amount;
    borrower_info.spendable += amount;

    store_borrower_info(&mut deps.storage, &borrower_raw, &borrower_info)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "deposit_collateral"),
            log("borrower", borrower.as_str()),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

/// Withdraw spendable collateral or a specified amount of collateral
/// Executor: borrower
pub fn withdraw_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Option<Uint256>,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;

    let borrower = env.message.sender;
    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);

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
        remove_borrower_info(&mut deps.storage, &borrower_raw);
    } else {
        store_borrower_info(&mut deps.storage, &borrower_raw, &borrower_info)?;
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.collateral_token)?,
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: borrower.clone(),
                amount: amount.into(),
            })?,
        })],
        log: vec![
            log("action", "withdraw_collateral"),
            log("borrower", borrower.as_str()),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

/// Decrease spendable collateral to lock
/// specified amount of collateral token
/// Executor: overseer
pub fn lock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    borrower: HumanAddr,
    amount: Uint256,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.overseer_contract {
        return Err(StdError::unauthorized());
    }

    let borrower_raw: CanonicalAddr = deps.api.canonical_address(&borrower)?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);
    if amount > borrower_info.spendable {
        return Err(StdError::generic_err(format!(
            "Lock amount cannot excceed the user's spendable amount: {}",
            borrower_info.spendable
        )));
    }

    borrower_info.spendable = borrower_info.spendable - amount;
    store_borrower_info(&mut deps.storage, &borrower_raw, &borrower_info)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "lock_collateral"),
            log("borrower", borrower),
            log("amount", amount),
        ],
        data: None,
    })
}

/// Increase spendable collateral to unlock
/// specified amount of collateral token
/// Executor: overseer
pub fn unlock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    borrower: HumanAddr,
    amount: Uint256,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.overseer_contract {
        return Err(StdError::unauthorized());
    }

    let borrower_raw: CanonicalAddr = deps.api.canonical_address(&borrower)?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);
    let borrowed_amt = borrower_info.balance - borrower_info.spendable;
    if amount > borrowed_amt {
        return Err(StdError::generic_err(format!(
            "Unlock amount cannot exceed locked amount: {}",
            borrowed_amt
        )));
    }

    borrower_info.spendable += amount;
    store_borrower_info(&mut deps.storage, &borrower_raw, &borrower_info)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "unlock_collateral"),
            log("borrower", borrower),
            log("amount", amount),
        ],
        data: None,
    })
}

pub fn liquidate_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    liquidator: HumanAddr,
    borrower: HumanAddr,
    amount: Uint256,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.overseer_contract {
        return Err(StdError::unauthorized());
    }

    let borrower_raw: CanonicalAddr = deps.api.canonical_address(&borrower)?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);
    let borrowed_amt = borrower_info.balance - borrower_info.spendable;
    if amount > borrowed_amt {
        return Err(StdError::generic_err(format!(
            "Liquidation amount cannot exceed locked amount: {}",
            borrowed_amt
        )));
    }

    borrower_info.balance = borrower_info.balance - amount;
    store_borrower_info(&mut deps.storage, &borrower_raw, &borrower_info)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.collateral_token)?,
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: deps.api.human_address(&config.liquidation_contract)?,
                amount: amount.into(),
                msg: Some(to_binary(&LiquidationCw20HookMsg::ExecuteBid {
                    liquidator: liquidator.clone(),
                    fee_address: Some(deps.api.human_address(&config.overseer_contract)?),
                    repay_address: Some(deps.api.human_address(&config.market_contract)?),
                })?),
            })?,
        })],
        log: vec![
            log("action", "liquidate_collateral"),
            log("liquidator", liquidator),
            log("borrower", borrower),
            log("amount", amount),
        ],
        data: None,
    })
}

pub fn query_borrower<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrower: HumanAddr,
) -> StdResult<BorrowerResponse> {
    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);
    Ok(BorrowerResponse {
        borrower,
        balance: borrower_info.balance,
        spendable: borrower_info.spendable,
    })
}

pub fn query_borrowers<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<BorrowersResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.canonical_address(&start_after)?)
    } else {
        None
    };

    let borrowers = read_borrowers(deps, start_after, limit)?;
    Ok(BorrowersResponse { borrowers })
}
