use crate::bid::{
    execute_bid, query_bid, query_bids_by_collateral, query_bids_by_user, retract_bid, submit_bid,
};
use crate::state::{read_config, store_config, Config};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation::{
    ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, LiquidationAmountResponse, QueryMsg,
};
use moneymarket::querier::query_tax_rate;
use moneymarket::tokens::TokensHuman;

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
            max_premium_rate: msg.max_premium_rate,
            liquidation_threshold: msg.liquidation_threshold,
            price_timeframe: msg.price_timeframe,
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
            max_premium_rate,
            liquidation_threshold,
            price_timeframe,
        } => update_config(
            deps,
            env,
            owner,
            oracle_contract,
            stable_denom,
            safe_ratio,
            bid_fee,
            max_premium_rate,
            liquidation_threshold,
            price_timeframe,
        ),
        HandleMsg::SubmitBid {
            collateral_token,
            premium_rate,
        } => submit_bid(deps, env, collateral_token, premium_rate),
        HandleMsg::RetractBid {
            collateral_token,
            amount,
        } => retract_bid(deps, env, collateral_token, amount),
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

                execute_bid(
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
    max_premium_rate: Option<Decimal256>,
    liquidation_threshold: Option<Uint256>,
    price_timeframe: Option<u64>,
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

    if let Some(max_premium_rate) = max_premium_rate {
        config.max_premium_rate = max_premium_rate;
    }

    if let Some(liquidation_threshold) = liquidation_threshold {
        config.liquidation_threshold = liquidation_threshold;
    }

    if let Some(price_timeframe) = price_timeframe {
        config.price_timeframe = price_timeframe;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse::default())
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
        QueryMsg::Bid {
            collateral_token,
            bidder,
        } => to_binary(&query_bid(deps, collateral_token, bidder)?),
        QueryMsg::BidsByUser {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_bids_by_user(deps, bidder, start_after, limit)?),
        QueryMsg::BidsByCollateral {
            collateral_token,
            start_after,
            limit,
        } => to_binary(&query_bids_by_collateral(
            deps,
            collateral_token,
            start_after,
            limit,
        )?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&config.owner)?,
        oracle_contract: deps.api.human_address(&config.oracle_contract)?,
        stable_denom: config.stable_denom,
        safe_ratio: config.safe_ratio,
        bid_fee: config.bid_fee,
        max_premium_rate: config.max_premium_rate,
        liquidation_threshold: config.liquidation_threshold,
        price_timeframe: config.price_timeframe,
    };

    Ok(resp)
}

fn query_liquidation_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrow_amount: Uint256,
    borrow_limit: Uint256,
    collaterals: TokensHuman,
    collateral_prices: Vec<Decimal256>,
) -> StdResult<LiquidationAmountResponse> {
    let config: Config = read_config(&deps.storage)?;

    // Safely collateralized check
    if borrow_amount <= borrow_limit {
        return Ok(LiquidationAmountResponse {
            collaterals: vec![],
        });
    }

    let mut collaterals_value = Uint256::zero();
    for c in collaterals.iter().zip(collateral_prices.iter()) {
        let (collateral, price) = c;
        let collateral_value = collateral.1 * *price;
        collaterals_value += collateral_value;
    }

    let tax_rate = query_tax_rate(&deps)?;

    let fee_deductor = (Decimal256::one() - config.max_premium_rate)
        * (Decimal256::one() - config.bid_fee)
        * (Decimal256::one() - tax_rate);

    // expected_repay_amount must be bigger than borrow_amount
    let expected_repay_amount = collaterals_value * fee_deductor;
    if expected_repay_amount <= borrow_amount {
        return Err(StdError::generic_err(
            "Cannot liquidate an undercollateralized loan",
        ));
    }

    // When collaterals_value is smaller than liquidation_threshold,
    // liquidate all collaterals
    let safe_borrow_amount = borrow_limit * config.safe_ratio;
    let liquidation_ratio = if collaterals_value < config.liquidation_threshold {
        Decimal256::from_uint256(borrow_amount) / Decimal256::from_uint256(expected_repay_amount)
    } else {
        Decimal256::from_uint256(borrow_amount - safe_borrow_amount)
            / Decimal256::from_uint256(expected_repay_amount - safe_borrow_amount)
    };

    // Cap the liquidation_ratio to 1
    let liquidation_ratio = std::cmp::min(Decimal256::one(), liquidation_ratio);
    Ok(LiquidationAmountResponse {
        collaterals: collaterals
            .iter()
            .zip(collateral_prices.iter())
            .map(|c| {
                let (collateral, _) = c;
                let mut collateral = collateral.clone();

                collateral.1 = collateral.1 * liquidation_ratio;
                collateral
            })
            .filter(|c| c.1 > Uint256::zero())
            .collect::<TokensHuman>(),
    })
}
