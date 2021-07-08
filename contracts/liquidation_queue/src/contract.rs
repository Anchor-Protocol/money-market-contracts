use crate::asserts::assert_max_slot;
use crate::bid::{activate_bids, claim_liquidations, execute_liquidation, retract_bid, submit_bid};
use crate::query::{
    query_bid, query_bid_pool, query_bid_pools, query_bids_by_user, query_config,
    query_liquidation_amount,
};
use crate::state::{
    read_collateral_info, read_config, store_collateral_info, store_config, CollateralInfo, Config,
};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{Cw20HookMsg, HandleMsg, InitMsg, QueryMsg};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: deps.api.canonical_address(&msg.owner)?,
            oracle_contract: deps.api.canonical_address(&msg.oracle_contract)?,
            stable_denom: msg.stable_denom,
            safe_ratio: msg.safe_ratio,
            bid_fee: msg.bid_fee,
            liquidation_threshold: msg.liquidation_threshold,
            price_timeframe: msg.price_timeframe,
            waiting_period: msg.waiting_period,
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::UpdateConfig {
            owner,
            oracle_contract,
            stable_denom,
            safe_ratio,
            bid_fee,
            liquidation_threshold,
            price_timeframe,
            waiting_period,
        } => update_config(
            deps,
            env,
            owner,
            oracle_contract,
            stable_denom,
            safe_ratio,
            bid_fee,
            liquidation_threshold,
            price_timeframe,
            waiting_period,
        ),
        HandleMsg::WhitelistCollateral {
            collateral_token,
            bid_threshold,
            max_slot,
        } => whitelist_collateral(deps, env, collateral_token, bid_threshold, max_slot),
        HandleMsg::SubmitBid {
            collateral_token,
            premium_slot,
        } => submit_bid(deps, env, collateral_token, premium_slot),
        HandleMsg::ActivateBids {
            collateral_token,
            bids_idx,
        } => activate_bids(deps, env, collateral_token, bids_idx),
        HandleMsg::RetractBid { bid_idx, amount } => retract_bid(deps, env, bid_idx, amount),
        HandleMsg::ClaimLiquidations {
            collateral_token,
            bids_idx,
        } => claim_liquidations(deps, env, collateral_token, bids_idx),
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    let contract_addr = env.message.sender.clone();
    if let Some(msg) = cw20_msg.msg.clone() {
        match from_binary(&msg)? {
            Cw20HookMsg::ExecuteBid {
                liquidator,
                repay_address,
                fee_address,
            } => {
                let collateral_token = contract_addr;
                let repay_address = repay_address.unwrap_or_else(|| cw20_msg.sender.clone());
                let fee_address = fee_address.unwrap_or_else(|| cw20_msg.sender.clone());

                execute_liquidation(
                    deps,
                    env,
                    liquidator,
                    repay_address,
                    fee_address,
                    collateral_token,
                    cw20_msg.amount.into(),
                )
            }
        }
    } else {
        Err(StdError::generic_err(
            "Invalid request: \"execute bid\" message not included in request",
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    oracle_contract: Option<HumanAddr>,
    stable_denom: Option<String>,
    safe_ratio: Option<Decimal256>,
    bid_fee: Option<Decimal256>,
    liquidation_threshold: Option<Uint256>,
    price_timeframe: Option<u64>,
    waiting_period: Option<u64>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(oracle_contract) = oracle_contract {
        config.oracle_contract = deps.api.canonical_address(&oracle_contract)?;
    }

    if let Some(stable_denom) = stable_denom {
        config.stable_denom = stable_denom;
    }

    if let Some(safe_ratio) = safe_ratio {
        config.safe_ratio = safe_ratio;
    }

    if let Some(bid_fee) = bid_fee {
        config.bid_fee = bid_fee;
    }

    if let Some(liquidation_threshold) = liquidation_threshold {
        config.liquidation_threshold = liquidation_threshold;
    }

    if let Some(price_timeframe) = price_timeframe {
        config.price_timeframe = price_timeframe;
    }

    if let Some(waiting_period) = waiting_period {
        config.waiting_period = waiting_period;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse::default())
}

pub fn whitelist_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    bid_threshold: Uint256,
    max_slot: u8,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    // fail if the collateral is already whitelisted
    if read_collateral_info(&deps.storage, &collateral_token_raw).is_ok() {
        return Err(StdError::generic_err("Collateral is already whitelisted"));
    }

    // assert max slot does note exceed cap
    assert_max_slot(max_slot)?;

    // save collateral info
    store_collateral_info(
        &mut deps.storage,
        &collateral_token_raw,
        &CollateralInfo {
            collateral_token: collateral_token_raw.clone(),
            max_slot,
            bid_threshold,
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "whitelist_collateral")],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::LiquidationAmount {
            borrow_amount,
            borrow_limit,
            collaterals,
            collateral_prices,
        } => to_binary(&query_liquidation_amount(
            deps,
            borrow_amount,
            borrow_limit,
            collaterals,
            collateral_prices,
        )?),
        QueryMsg::Bid { bid_idx } => to_binary(&query_bid(deps, bid_idx)?),
        QueryMsg::BidsByUser {
            collateral_token,
            bidder,
            start_after,
            limit,
        } => to_binary(&query_bids_by_user(
            deps,
            collateral_token,
            bidder,
            start_after,
            limit,
        )?),
        QueryMsg::BidPool {
            collateral_token,
            bid_slot,
        } => to_binary(&query_bid_pool(deps, collateral_token, bid_slot)?),
        QueryMsg::BidPoolsByCollateral {
            collateral_token,
            start_after,
            limit,
        } => to_binary(&query_bid_pools(
            deps,
            collateral_token,
            start_after,
            limit,
        )?),
    }
}
