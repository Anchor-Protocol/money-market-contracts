use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, Querier, StdError, StdResult, Storage, WasmMsg,
};
use cw20::Cw20HandleMsg;
use moneymarket::{deduct_tax, query_price, PriceResponse};

use crate::msg::{BidResponse, BidsResponse};
use crate::state::{
    read_bid, read_bids_by_collateral, read_bids_by_user, read_config, remove_bid, store_bid, Bid,
    Config,
};

pub fn submit_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    premium_rate: Decimal256,
) -> HandleResult {
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    let bidder_raw = deps.api.canonical_address(&env.message.sender)?;
    if read_bid(&deps.storage, &bidder_raw, &collateral_token_raw).is_ok() {
        return Err(StdError::generic_err("Bid already exists"));
    }

    let config: Config = read_config(&deps.storage)?;
    if config.max_premium_rate < premium_rate {
        return Err(StdError::generic_err(
            "premium_rate cannot be bigger than max_premium_rate",
        ));
    }

    let amount: Uint256 = Uint256::from(
        env.message
            .sent_funds
            .iter()
            .find(|c| c.denom == config.stable_denom)
            .map(|c| c.amount)
            .ok_or_else(|| StdError::generic_err("Must provide stable_denom asset"))?,
    );

    store_bid(
        &mut deps.storage,
        &bidder_raw,
        &collateral_token_raw,
        Bid {
            amount,
            premium_rate,
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "submit_bid"),
            log("collateral_token", collateral_token),
            log("amount", amount),
        ],
        data: None,
    })
}

pub fn retract_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    amount: Option<Uint256>,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    let bidder_raw = deps.api.canonical_address(&env.message.sender)?;
    let bid: Bid = read_bid(&deps.storage, &bidder_raw, &collateral_token_raw)?;

    let amount = amount.unwrap_or(bid.amount);
    if amount > bid.amount {
        return Err(StdError::generic_err(
            "Cannot retract bigger amount than the bid balance",
        ));
    }

    store_bid(
        &mut deps.storage,
        &bidder_raw,
        &collateral_token_raw,
        Bid {
            amount: bid.amount - amount,
            ..bid
        },
    )?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: env.message.sender.clone(),
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: config.stable_denom,
                    amount: amount.into(),
                },
            )?],
        })],
        log: vec![
            log("action", "retract_bid"),
            log("collateral_token", collateral_token),
            log("bidder", env.message.sender),
            log("amount", amount),
        ],
        data: None,
    })
}

pub fn execute_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    liquidator: HumanAddr,
    repay_address: HumanAddr,
    fee_address: HumanAddr,
    collateral_token: HumanAddr,
    amount: Uint256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    let bidder_raw = deps.api.canonical_address(&liquidator)?;
    let bid: Bid = read_bid(&deps.storage, &bidder_raw, &collateral_token_raw)?;

    let oracle_contract = deps.api.human_address(&config.oracle_contract)?;
    let price: PriceResponse = query_price(
        &deps,
        &oracle_contract,
        collateral_token.to_string(),
        config.stable_denom.clone(),
    )?;

    let collateral_value = amount * price.rate;
    let required_stable = collateral_value
        * (Decimal256::one() - std::cmp::min(bid.premium_rate, config.max_premium_rate));
    if required_stable > bid.amount {
        return Err(StdError::generic_err(
            "Bid amount is smaller than required_stable",
        ));
    }

    // Update bid
    if bid.amount == required_stable {
        remove_bid(&mut deps.storage, &bidder_raw, &collateral_token_raw);
    } else {
        store_bid(
            &mut deps.storage,
            &bidder_raw,
            &collateral_token_raw,
            Bid {
                amount: bid.amount - required_stable,
                ..bid
            },
        )?;
    }

    let bid_fee = required_stable * config.bid_fee;
    let repay_amount = required_stable - bid_fee;

    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: repay_address,
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: config.stable_denom.clone(),
                        amount: repay_amount.into(),
                    },
                )?],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address,
                to_address: fee_address,
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: config.stable_denom.clone(),
                        amount: bid_fee.into(),
                    },
                )?],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: collateral_token.clone(),
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: liquidator,
                    amount: amount.into(),
                })?,
            }),
        ],
        log: vec![
            log("action", "execute_bid"),
            log("stable_denom", config.stable_denom),
            log("repay_amount", repay_amount),
            log("bid_fee", bid_fee),
            log("collateral_token", collateral_token),
            log("collateral_amount", amount),
        ],
        data: None,
    })
}

pub fn query_bid<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    collateral_token: HumanAddr,
    bidder: HumanAddr,
) -> StdResult<BidResponse> {
    let bid: Bid = read_bid(
        &deps.storage,
        &deps.api.canonical_address(&bidder)?,
        &deps.api.canonical_address(&collateral_token)?,
    )?;

    Ok(BidResponse {
        collateral_token,
        bidder,
        amount: bid.amount,
        premium_rate: bid.premium_rate,
    })
}

pub fn query_bids_by_user<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    bidder: HumanAddr,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.canonical_address(&start_after)?)
    } else {
        None
    };

    let bids: Vec<BidResponse> = read_bids_by_user(
        &deps,
        &deps.api.canonical_address(&bidder)?,
        start_after,
        limit,
    )?;

    Ok(BidsResponse { bids })
}

pub fn query_bids_by_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    collateral_token: HumanAddr,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.canonical_address(&start_after)?)
    } else {
        None
    };

    let bids: Vec<BidResponse> = read_bids_by_collateral(
        &deps,
        &deps.api.canonical_address(&collateral_token)?,
        start_after,
        limit,
    )?;

    Ok(BidsResponse { bids })
}
