use crate::error::ContractError;
use crate::state::{
    read_bid, read_bids_by_collateral, read_bids_by_user, read_config, remove_bid, store_bid, Bid,
    Config,
};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use moneymarket::liquidation::{BidResponse, BidsResponse};
use moneymarket::oracle::PriceResponse;
use moneymarket::querier::{deduct_tax, query_price, TimeConstraints};

pub fn submit_bid(
    deps: DepsMut,
    info: MessageInfo,
    collateral_token: Addr,
    premium_rate: Decimal256,
) -> Result<Response, ContractError> {
    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    let bidder_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    if read_bid(deps.storage, &bidder_raw, &collateral_token_raw).is_ok() {
        return Err(ContractError::AlreadyBidForCollateral(collateral_token));
    }

    let config: Config = read_config(deps.storage)?;
    if config.max_premium_rate < premium_rate {
        return Err(ContractError::PremiumExceedsMaxPremium(
            config.max_premium_rate.to_string(),
        ));
    }

    let amount: Uint256 = Uint256::from(
        info.funds
            .iter()
            .find(|c| c.denom == config.stable_denom)
            .map(|c| c.amount)
            .ok_or(ContractError::AssetNotProvided(config.stable_denom))?,
    );

    store_bid(
        deps.storage,
        &bidder_raw,
        &collateral_token_raw,
        Bid {
            amount,
            premium_rate,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "submit_bid"),
        attr("collateral_token", collateral_token),
        attr("amount", amount),
    ]))
}

pub fn retract_bid(
    deps: DepsMut,
    info: MessageInfo,
    collateral_token: Addr,
    amount: Option<Uint256>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    let bidder_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    let bid: Bid = read_bid(deps.storage, &bidder_raw, &collateral_token_raw)?;

    let amount = amount.unwrap_or(bid.amount);
    if amount > bid.amount {
        return Err(ContractError::RetractExceedsBid(bid.amount.into()));
    }

    if amount == bid.amount {
        remove_bid(deps.storage, &bidder_raw, &collateral_token_raw);
    } else {
        store_bid(
            deps.storage,
            &bidder_raw,
            &collateral_token_raw,
            Bid {
                amount: bid.amount - amount,
                ..bid
            },
        )?;
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom,
                    amount: amount.into(),
                },
            )?],
        }))
        .add_attributes(vec![
            attr("action", "retract_bid"),
            attr("collateral_token", collateral_token),
            attr("bidder", info.sender),
            attr("amount", amount),
        ]))
}

pub fn execute_bid(
    deps: DepsMut,
    env: Env,
    liquidator: Addr,
    repay_address: Addr,
    fee_address: Addr,
    collateral_token: Addr,
    amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    let bidder_raw = deps.api.addr_canonicalize(liquidator.as_str())?;
    let bid: Bid = read_bid(deps.storage, &bidder_raw, &collateral_token_raw)?;

    let oracle_contract = deps.api.addr_humanize(&config.oracle_contract)?;
    let price: PriceResponse = query_price(
        deps.as_ref(),
        oracle_contract,
        collateral_token.to_string(),
        config.stable_denom.clone(),
        Some(TimeConstraints {
            block_time: env.block.time.seconds(),
            valid_timeframe: config.price_timeframe,
        }),
    )?;

    let collateral_value = amount * price.rate;
    let required_stable = collateral_value
        * (Decimal256::one() - std::cmp::min(bid.premium_rate, config.max_premium_rate));
    if required_stable > bid.amount {
        return Err(ContractError::InsufficientBidBalance(
            required_stable.into(),
        ));
    }

    // Update bid
    if bid.amount == required_stable {
        remove_bid(deps.storage, &bidder_raw, &collateral_token_raw);
    } else {
        store_bid(
            deps.storage,
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

    let mut messages: Vec<CosmosMsg> = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral_token.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: liquidator.to_string(),
                amount: amount.into(),
            })?,
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: repay_address.to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom.clone(),
                    amount: repay_amount.into(),
                },
            )?],
        }),
    ];

    if !bid_fee.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: fee_address.to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom.clone(),
                    amount: bid_fee.into(),
                },
            )?],
        }));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "execute_bid"),
        attr("stable_denom", config.stable_denom),
        attr("repay_amount", repay_amount),
        attr("bid_fee", bid_fee),
        attr("collateral_token", collateral_token),
        attr("collateral_amount", amount),
    ]))
}

pub fn query_bid(deps: Deps, collateral_token: Addr, bidder: Addr) -> StdResult<BidResponse> {
    let bid: Bid = read_bid(
        deps.storage,
        &deps.api.addr_canonicalize(bidder.as_str())?,
        &deps.api.addr_canonicalize(collateral_token.as_str())?,
    )?;

    Ok(BidResponse {
        collateral_token: collateral_token.to_string(),
        bidder: bidder.to_string(),
        amount: bid.amount,
        premium_rate: bid.premium_rate,
    })
}

pub fn query_bids_by_user(
    deps: Deps,
    bidder: Addr,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.addr_canonicalize(start_after.as_str())?)
    } else {
        None
    };

    let bids: Vec<BidResponse> = read_bids_by_user(
        deps,
        &deps.api.addr_canonicalize(bidder.as_str())?,
        start_after,
        limit,
    )?;

    Ok(BidsResponse { bids })
}

pub fn query_bids_by_collateral(
    deps: Deps,
    collateral_token: Addr,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.addr_canonicalize(start_after.as_str())?)
    } else {
        None
    };

    let bids: Vec<BidResponse> = read_bids_by_collateral(
        deps,
        &deps.api.addr_canonicalize(collateral_token.as_str())?,
        start_after,
        limit,
    )?;

    Ok(BidsResponse { bids })
}
