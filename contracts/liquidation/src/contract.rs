#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::bid::{
    execute_bid, query_bid, query_bids_by_collateral, query_bids_by_user, retract_bid, submit_bid,
};
use crate::error::ContractError;
use crate::state::{read_config, store_config, Config};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw20::Cw20ReceiveMsg;
use moneymarket::common::optional_addr_validate;
use moneymarket::liquidation::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LiquidationAmountResponse, QueryMsg,
};
use moneymarket::querier::query_tax_rate;
use moneymarket::tokens::TokensHuman;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(&msg.owner)?,
            oracle_contract: deps.api.addr_canonicalize(&msg.oracle_contract)?,
            stable_denom: msg.stable_denom,
            safe_ratio: msg.safe_ratio,
            bid_fee: msg.bid_fee,
            max_premium_rate: msg.max_premium_rate,
            liquidation_threshold: msg.liquidation_threshold,
            price_timeframe: msg.price_timeframe,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            oracle_contract,
            stable_denom,
            safe_ratio,
            bid_fee,
            max_premium_rate,
            liquidation_threshold,
            price_timeframe,
        } => {
            let api = deps.api;
            update_config(
                deps,
                info,
                optional_addr_validate(api, owner)?,
                optional_addr_validate(api, oracle_contract)?,
                stable_denom,
                safe_ratio,
                bid_fee,
                max_premium_rate,
                liquidation_threshold,
                price_timeframe,
            )
        }
        ExecuteMsg::SubmitBid {
            collateral_token,
            premium_rate,
        } => {
            let api = deps.api;
            submit_bid(
                deps,
                info,
                api.addr_validate(&collateral_token)?,
                premium_rate,
            )
        }
        ExecuteMsg::RetractBid {
            collateral_token,
            amount,
        } => {
            let api = deps.api;
            retract_bid(deps, info, api.addr_validate(&collateral_token)?, amount)
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::ExecuteBid {
            liquidator,
            repay_address,
            fee_address,
        }) => {
            let collateral_token = contract_addr.to_string();
            let repay_address = repay_address.unwrap_or_else(|| cw20_msg.sender.clone());
            let fee_address = fee_address.unwrap_or_else(|| cw20_msg.sender.clone());

            let api = deps.api;

            execute_bid(
                deps,
                env,
                api.addr_validate(&liquidator)?,
                api.addr_validate(&repay_address)?,
                api.addr_validate(&fee_address)?,
                api.addr_validate(&collateral_token)?,
                cw20_msg.amount.into(),
            )
        }
        _ => Err(ContractError::MissingExecuteBidHook {}),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    oracle_contract: Option<Addr>,
    stable_denom: Option<String>,
    safe_ratio: Option<Decimal256>,
    bid_fee: Option<Decimal256>,
    max_premium_rate: Option<Decimal256>,
    liquidation_threshold: Option<Uint256>,
    price_timeframe: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(&owner.to_string())?;
    }

    if let Some(oracle_contract) = oracle_contract {
        config.oracle_contract = deps.api.addr_canonicalize(&oracle_contract.to_string())?;
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

    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
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
        } => {
            let collateral_addr = deps.api.addr_validate(&collateral_token)?;
            let bidder_addr = deps.api.addr_validate(&bidder)?;
            to_binary(&query_bid(deps, collateral_addr, bidder_addr)?)
        }
        QueryMsg::BidsByUser {
            bidder,
            start_after,
            limit,
        } => {
            let bidder_addr = deps.api.addr_validate(&bidder)?;
            let start_after_addr = optional_addr_validate(deps.api, start_after)?;
            to_binary(&query_bids_by_user(
                deps,
                bidder_addr,
                start_after_addr,
                limit,
            )?)
        }
        QueryMsg::BidsByCollateral {
            collateral_token,
            start_after,
            limit,
        } => {
            let collateral_addr = deps.api.addr_validate(&collateral_token)?;
            let start_after_addr = optional_addr_validate(deps.api, start_after)?;
            to_binary(&query_bids_by_collateral(
                deps,
                collateral_addr,
                start_after_addr,
                limit,
            )?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        oracle_contract: deps.api.addr_humanize(&config.oracle_contract)?.to_string(),
        stable_denom: config.stable_denom,
        safe_ratio: config.safe_ratio,
        bid_fee: config.bid_fee,
        max_premium_rate: config.max_premium_rate,
        liquidation_threshold: config.liquidation_threshold,
        price_timeframe: config.price_timeframe,
    };

    Ok(resp)
}

fn query_liquidation_amount(
    deps: Deps,
    borrow_amount: Uint256,
    borrow_limit: Uint256,
    collaterals: TokensHuman,
    collateral_prices: Vec<Decimal256>,
) -> StdResult<LiquidationAmountResponse> {
    let config: Config = read_config(deps.storage)?;

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

    let tax_rate = query_tax_rate(deps)?;

    let fee_deductor = (Decimal256::one() - config.max_premium_rate)
        * (Decimal256::one() - config.bid_fee)
        * (Decimal256::one() - tax_rate);

    // expected_repay_amount must be bigger than borrow_amount
    // else force liquidate all collaterals
    let expected_repay_amount = collaterals_value * fee_deductor;
    if expected_repay_amount <= borrow_amount {
        return Ok(LiquidationAmountResponse { collaterals });
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
