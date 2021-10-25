use crate::asserts::{assert_activate_status, assert_withdraw_amount};
use crate::querier::query_collateral_whitelist_info;
use crate::state::{
    pop_bid_idx, read_bid, read_bid_pool, read_bids_by_user, read_collateral_info, read_config,
    read_epoch_scale_sum, read_or_create_bid_pool, read_total_bids, remove_bid, store_bid,
    store_bid_pool, store_epoch_scale_sum, store_total_bids, Bid, BidPool, CollateralInfo, Config,
};
use bigint::U256;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, BankMsg, CanonicalAddr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use moneymarket::oracle::PriceResponse;
use moneymarket::querier::{deduct_tax, query_price, TimeConstraints};

/// Stable asset is submitted to create a bid record. If available bids for the collateral is under
/// the threshold, the bid is activated. Bids are not used for liquidations until activated
pub fn submit_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_token: String,
    premium_slot: u8,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let collateral_token_raw: CanonicalAddr = deps.api.addr_canonicalize(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(deps.storage, &collateral_token_raw)?;
    let bidder_raw = deps.api.addr_canonicalize(info.sender.as_str())?;

    let amount: Uint256 = info
        .funds
        .iter()
        .map(|item| {
            if item.denom != config.stable_denom {
                Err(StdError::generic_err(format!(
                    "Invalid asset provided, only {} allowed",
                    config.stable_denom
                )))
            } else {
                Ok(item.amount)
            }
        })
        .last()
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "No {} assets have been provided",
                config.stable_denom
            ))
        })??
        .into();

    // read or create bid_pool, make sure slot is valid
    let mut bid_pool: BidPool =
        read_or_create_bid_pool(deps.storage, &collateral_info, premium_slot)?;

    // create bid object
    let bid_idx: Uint128 = pop_bid_idx(deps.storage)?;
    let mut bid = Bid {
        idx: bid_idx,
        bidder: bidder_raw,
        collateral_token: collateral_token_raw.clone(),
        product_snapshot: Decimal256::one(),
        amount,
        premium_slot,
        sum_snapshot: Decimal256::zero(),
        pending_liquidated_collateral: Uint256::zero(),
        wait_end: None,
        epoch_snapshot: Uint128::zero(),
        scale_snapshot: Uint128::zero(),
    };

    // if available bids is lower than bid_threshold, directly activate bid
    let available_bids: Uint256 =
        read_total_bids(deps.storage, &collateral_token_raw).unwrap_or_default();
    if available_bids < collateral_info.bid_threshold {
        // update bid and bid pool, add new share and pool indexes to bid
        process_bid_activation(&mut bid, &mut bid_pool, amount);

        // store bid_pool
        store_bid_pool(deps.storage, &collateral_token_raw, premium_slot, &bid_pool)?;

        // increase total bid amount
        store_total_bids(deps.storage, &collateral_token_raw, available_bids + amount)?;
    } else {
        // calculate wait_end from current time
        bid.wait_end = Some(env.block.time.plus_seconds(config.waiting_period).seconds());
    };

    // save to storage
    store_bid(deps.storage, bid_idx, &bid)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "submit_bid"),
        attr("bid_idx", bid_idx),
        attr("amount", amount),
    ]))
}

/// After bids are submitted, need to execute the activation after wait_period expires
/// Bids are not used for liquidations until activated
pub fn activate_bids(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_token: String,
    bids_idx: Option<Vec<Uint128>>,
) -> StdResult<Response> {
    let sender_raw: CanonicalAddr = deps.api.addr_canonicalize(info.sender.as_str())?;
    let collateral_token_raw: CanonicalAddr = deps.api.addr_canonicalize(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(deps.storage, &collateral_token_raw)?;
    let mut available_bids: Uint256 =
        read_total_bids(deps.storage, &collateral_token_raw).unwrap_or_default();

    let bids: Vec<Bid> = if let Some(bids_idx) = &bids_idx {
        bids_idx
            .iter()
            .map(|idx| read_bid(deps.storage, *idx))
            .collect::<StdResult<Vec<Bid>>>()?
    } else {
        read_bids_by_user(deps.storage, &collateral_token_raw, &sender_raw, None, None)?
            .into_iter()
            .filter(|b| b.wait_end.is_some())
            .collect::<Vec<Bid>>()
    };

    let mut total_activated_amount = Uint256::zero();
    for mut bid in bids.into_iter() {
        if bid.bidder != sender_raw {
            return Err(StdError::generic_err("unauthorized"));
        }
        if bid.collateral_token != collateral_token_raw {
            return Err(StdError::generic_err("Bid collateral token doesn't match"));
        }
        let mut bid_pool: BidPool =
            read_bid_pool(deps.storage, &bid.collateral_token, bid.premium_slot)?;

        let amount_to_activate = bid.amount;

        // assert that the bid is inactive and wait period has expired
        if let Err(err) =
            assert_activate_status(&bid, &env, available_bids, collateral_info.bid_threshold)
        {
            if bids_idx.is_some() {
                // if the user provided the idx to activate, we should return error to notify the user
                return Err(err);
            } else {
                // otherwise just skip this bid
                continue;
            }
        }

        // update bid and bid pool, add new share and pool indexes to bid
        process_bid_activation(&mut bid, &mut bid_pool, amount_to_activate);

        // save to storage
        store_bid(deps.storage, bid.idx, &bid)?;
        store_bid_pool(
            deps.storage,
            &bid.collateral_token,
            bid.premium_slot,
            &bid_pool,
        )?;

        total_activated_amount += amount_to_activate;
        available_bids += amount_to_activate;
    }

    store_total_bids(deps.storage, &collateral_token_raw, available_bids)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "activate_bids"),
        attr("amount", total_activated_amount),
    ]))
}

/// Bid owners can withdraw the ramaning bid amount at any time
pub fn retract_bid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    bid_idx: Uint128,
    amount: Option<Uint256>,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let sender_raw: CanonicalAddr = deps.api.addr_canonicalize(info.sender.as_str())?;
    let mut bid: Bid = read_bid(deps.storage, bid_idx)?;
    let collateral_token_raw: CanonicalAddr = bid.collateral_token.clone();

    if bid.bidder != sender_raw {
        return Err(StdError::generic_err("unauthorized"));
    }

    // check if bid is active or waiting
    let withdraw_amount: Uint256 = if bid.wait_end.is_some() {
        // waiting bid amount can be withdrawn without restriction
        let waiting_withdraw_amount = assert_withdraw_amount(amount, bid.amount)?;
        if waiting_withdraw_amount == bid.amount {
            remove_bid(deps.storage, bid.idx)?;
        } else {
            bid.amount = bid.amount - waiting_withdraw_amount;
            store_bid(deps.storage, bid.idx, &bid)?;
        }

        waiting_withdraw_amount
    } else {
        let available_bids: Uint256 = read_total_bids(deps.storage, &collateral_token_raw)?;
        let mut bid_pool: BidPool =
            read_bid_pool(deps.storage, &collateral_token_raw, bid.premium_slot)?;

        // calculate spent and reward until this moment
        let (withdrawable_amount, residue_bid) = calculate_remaining_bid(&bid, &bid_pool)?;
        let (liquidated_collateral, residue_collateral) =
            calculate_liquidated_collateral(deps.storage, &bid)?;

        // accumulate pending reward to be claimed later
        bid.pending_liquidated_collateral += liquidated_collateral;

        // stack residues, will give it to next claimer if it becomes bigger than 1.0
        bid_pool.residue_collateral += residue_collateral;
        bid_pool.residue_bid += residue_bid;

        // check requested amount
        let withdraw_amount: Uint256 = assert_withdraw_amount(amount, withdrawable_amount)?;

        // remove or update bid
        if withdraw_amount == withdrawable_amount && bid.pending_liquidated_collateral.is_zero() {
            remove_bid(deps.storage, bid_idx)?;
        } else {
            store_bid(
                deps.storage,
                bid.idx,
                &Bid {
                    amount: withdrawable_amount - withdraw_amount,
                    product_snapshot: bid_pool.product_snapshot,
                    sum_snapshot: bid_pool.sum_snapshot,
                    scale_snapshot: bid_pool.current_scale,
                    ..bid
                },
            )?;
        }

        // update available bid amount
        bid_pool.total_bid_amount = bid_pool.total_bid_amount - withdraw_amount;

        // claim residue bids if it is bigger than 1.0
        let refund_amount = withdraw_amount + claim_bid_residue(&mut bid_pool);

        store_bid_pool(
            deps.storage,
            &collateral_token_raw,
            bid.premium_slot,
            &bid_pool,
        )?;
        store_total_bids(
            deps.storage,
            &collateral_token_raw,
            available_bids - withdraw_amount,
        )?;

        refund_amount
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    if !withdraw_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom,
                    amount: withdraw_amount.into(),
                },
            )?],
        }));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "retract_bid"),
        attr("bid_idx", bid_idx),
        attr("amount", withdraw_amount),
    ]))
}

/// Overseer executes the liquidation providing a whitelisted collateral.
/// This operation returns a repay_amount based on the available bids on each
/// premium slot, consuming bids from lowest to higher premium slots
#[allow(clippy::too_many_arguments)]
pub fn execute_liquidation(
    deps: DepsMut,
    env: Env,
    sender: String,
    liquidator: String,
    repay_address: String,
    fee_address: String,
    collateral_token: String,
    amount: Uint256,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(deps.storage, &collateral_token_raw)?;
    let available_bids: Uint256 = read_total_bids(deps.storage, &collateral_token_raw)?;

    // only collateral token custody can execute liquidations
    let overseer = deps.api.addr_humanize(&config.overseer)?;
    let custody_contract = query_collateral_whitelist_info(
        &deps.querier,
        overseer.to_string(),
        collateral_token.to_string(),
    )?
    .custody_contract;
    if sender != custody_contract {
        return Err(StdError::generic_err(
            "Unauthorized: only custody contract can execute liquidations",
        ));
    }

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

    let mut remaining_collateral_to_liquidate = amount;
    let mut repay_amount = Uint256::zero();
    let mut filled: bool = false;
    for slot in 0..collateral_info.max_slot + 1 {
        let mut bid_pool: BidPool = match read_bid_pool(deps.storage, &collateral_token_raw, slot) {
            Ok(bid_pool) => bid_pool,
            Err(_) => continue,
        };
        if bid_pool.total_bid_amount.is_zero() {
            continue;
        };

        let (pool_repay_amount, pool_liquidated_collateral) = execute_pool_liquidation(
            deps.storage,
            &mut bid_pool,
            &collateral_token_raw,
            slot,
            remaining_collateral_to_liquidate,
            price.rate,
            &mut filled,
        )?;

        store_bid_pool(deps.storage, &collateral_token_raw, slot, &bid_pool)?;

        repay_amount += pool_repay_amount;

        if filled {
            remaining_collateral_to_liquidate = Uint256::zero();
            break;
        } else {
            remaining_collateral_to_liquidate =
                remaining_collateral_to_liquidate - pool_liquidated_collateral;
        }
    }

    if !remaining_collateral_to_liquidate.is_zero() {
        return Err(StdError::generic_err(
            "Not enough bids to execute this liquidation",
        ));
    }

    store_total_bids(
        deps.storage,
        &collateral_token_raw,
        available_bids - repay_amount,
    )?;

    let bid_fee = repay_amount * config.bid_fee;
    let liquidator_fee = repay_amount * config.liquidator_fee;
    let repay_amount = repay_amount - bid_fee - liquidator_fee;

    let mut messages: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: repay_address,
        amount: vec![deduct_tax(
            deps.as_ref(),
            Coin {
                denom: config.stable_denom.clone(),
                amount: repay_amount.into(),
            },
        )?],
    })];

    if !bid_fee.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: fee_address,
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom.clone(),
                    amount: bid_fee.into(),
                },
            )?],
        }));
    }
    if !liquidator_fee.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: liquidator,
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom.clone(),
                    amount: liquidator_fee.into(),
                },
            )?],
        }));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "execute_bid"),
        attr("stable_denom", config.stable_denom),
        attr("repay_amount", repay_amount),
        attr("bid_fee", bid_fee),
        attr("liquidator_fee", liquidator_fee),
        attr("collateral_token", collateral_token),
        attr("collateral_amount", amount),
    ]))
}

/// Bid owner can claim their share of the liquidated collateral until the
/// bid is consumed
pub fn claim_liquidations(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collateral_token: String,
    bids_idx: Option<Vec<Uint128>>,
) -> StdResult<Response> {
    let sender_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;

    let bids: Vec<Bid> = if let Some(bids_idx) = bids_idx {
        bids_idx
            .iter()
            .map(|idx| read_bid(deps.storage, *idx))
            .collect::<StdResult<Vec<Bid>>>()?
    } else {
        read_bids_by_user(deps.storage, &collateral_token_raw, &sender_raw, None, None)?
    };

    let mut claim_amount = Uint256::zero();
    for bid in bids.into_iter() {
        if bid.bidder != sender_raw {
            return Err(StdError::generic_err("unauthorized"));
        }
        if bid.collateral_token != collateral_token_raw {
            return Err(StdError::generic_err("Bid collateral token doesn't match"));
        }
        if bid.wait_end.is_some() {
            // bid not activated
            continue;
        }

        let mut bid_pool: BidPool =
            read_bid_pool(deps.storage, &bid.collateral_token, bid.premium_slot)?;

        // calculate remaining bid amount
        let (remaining_bid, residue_bid) = calculate_remaining_bid(&bid, &bid_pool)?;

        // calculate liquidated collateral
        let (liquidated_collateral, residue_collateral) =
            calculate_liquidated_collateral(deps.storage, &bid)?;

        // keep residues
        bid_pool.residue_collateral += residue_collateral;
        bid_pool.residue_bid += residue_bid;

        // get claimable amount
        claim_amount += bid.pending_liquidated_collateral
            + liquidated_collateral
            + claim_col_residue(&mut bid_pool);

        // store bid_pool to update residue
        store_bid_pool(
            deps.storage,
            &collateral_token_raw,
            bid.premium_slot,
            &bid_pool,
        )?;

        // check if bid has been consumed, include 1 for rounding
        if remaining_bid <= Uint256::one() {
            remove_bid(deps.storage, bid.idx)?;
        } else {
            store_bid(
                deps.storage,
                bid.idx,
                &Bid {
                    amount: remaining_bid,
                    product_snapshot: bid_pool.product_snapshot,
                    sum_snapshot: bid_pool.sum_snapshot,
                    scale_snapshot: bid_pool.current_scale,
                    pending_liquidated_collateral: Uint256::zero(),
                    ..bid
                },
            )?;
        }
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    if !claim_amount.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral_token.clone(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: claim_amount.into(),
            })?,
        }));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "claim_liquidations"),
        attr("collateral_token", collateral_token),
        attr("collateral_amount", claim_amount),
    ]))
}

fn process_bid_activation(bid: &mut Bid, bid_pool: &mut BidPool, amount: Uint256) {
    bid.product_snapshot = bid_pool.product_snapshot;
    bid.sum_snapshot = bid_pool.sum_snapshot;
    bid.wait_end = None;
    bid.scale_snapshot = bid_pool.current_scale;
    bid.epoch_snapshot = bid_pool.current_epoch;

    bid_pool.total_bid_amount += amount;
}

/// On each collateral execution the product_snapshot and sum_snapshot are updated
/// to track the expense and reward distribution for biders in the pool
/// More details:
/// https://github.com/liquity/liquity/blob/master/papers/Scalable_Reward_Distribution_with_Compounding_Stakes.pdf
#[allow(clippy::too_many_arguments)]
fn execute_pool_liquidation(
    storage: &mut dyn Storage,
    bid_pool: &mut BidPool,
    collateral_token: &CanonicalAddr,
    premium_slot: u8,
    collateral_to_liquidate: Uint256,
    price: Decimal256,
    filled: &mut bool,
) -> StdResult<(Uint256, Uint256)> {
    let premium_price = price * (Decimal256::one() - bid_pool.premium_rate);
    let mut pool_collateral_to_liquidate = collateral_to_liquidate;
    let mut pool_required_stable = pool_collateral_to_liquidate * premium_price;

    if pool_required_stable > bid_pool.total_bid_amount {
        pool_required_stable = bid_pool.total_bid_amount;
        pool_collateral_to_liquidate = pool_required_stable / premium_price;
    } else {
        *filled = true;
    }

    // E / D
    let col_per_bid: Decimal256 = Decimal256::from_uint256(pool_collateral_to_liquidate)
        / Decimal256::from_uint256(bid_pool.total_bid_amount);

    // Q / D
    let expense_per_bid: Decimal256 = Decimal256::from_uint256(pool_required_stable)
        / Decimal256::from_uint256(bid_pool.total_bid_amount);

    ///////// Update sum /////////
    // E / D * P
    let sum = bid_pool.product_snapshot * col_per_bid;

    // S + E / D * P
    bid_pool.sum_snapshot += sum;
    bid_pool.total_bid_amount = bid_pool.total_bid_amount - pool_required_stable;

    // save reward sum for current epoch and scale
    store_epoch_scale_sum(
        storage,
        collateral_token,
        premium_slot,
        bid_pool.current_epoch,
        bid_pool.current_scale,
        bid_pool.sum_snapshot,
    )?;

    ///////// Update product /////////
    // Check if the pool is emptied, if it is, reset (P = 1, S = 0)
    if expense_per_bid == Decimal256::one() {
        bid_pool.sum_snapshot = Decimal256::zero();
        bid_pool.product_snapshot = Decimal256::one();
        bid_pool.current_scale = Uint128::zero();

        bid_pool.current_epoch += Uint128::from(1u128);
    } else {
        // 1 - Q / D
        let product = Decimal256::one() - expense_per_bid;

        // check if scale needs to be increased (in case product truncates to zero)
        let new_product = bid_pool.product_snapshot * product;
        bid_pool.product_snapshot = if new_product < Decimal256(U256::from(1_000_000_000u64)) {
            bid_pool.current_scale += Uint128::from(1u128);

            Decimal256(bid_pool.product_snapshot.0 * U256::from(1_000_000_000u64)) * product
        } else {
            new_product
        };
    }
    println!("product: {}", bid_pool.product_snapshot);
    Ok((pool_required_stable, pool_collateral_to_liquidate))
}

pub(crate) fn calculate_remaining_bid(
    bid: &Bid,
    bid_pool: &BidPool,
) -> StdResult<(Uint256, Decimal256)> {
    let scale_diff: Uint128 = bid_pool.current_scale.checked_sub(bid.scale_snapshot)?;
    let epoch_diff: Uint128 = bid_pool.current_epoch.checked_sub(bid.epoch_snapshot)?;

    let remaining_bid_dec: Decimal256 = if !epoch_diff.is_zero() {
        // pool was emptied, return 0
        Decimal256::zero()
    } else if scale_diff.is_zero() {
        Decimal256::from_uint256(bid.amount) * bid_pool.product_snapshot / bid.product_snapshot
    } else if scale_diff == Uint128::from(1u128) {
        // product has been scaled
        let scaled_remaining_bid =
            Decimal256::from_uint256(bid.amount) * bid_pool.product_snapshot / bid.product_snapshot;

        Decimal256(scaled_remaining_bid.0 / U256::from(1_000_000_000u64))
    } else {
        Decimal256::zero()
    };

    let remaining_bid = remaining_bid_dec * Uint256::one();
    // stacks the residue when converting to integer
    let bid_residue = remaining_bid_dec - Decimal256::from_uint256(remaining_bid);

    Ok((remaining_bid, bid_residue))
}

pub(crate) fn calculate_liquidated_collateral(
    storage: &dyn Storage,
    bid: &Bid,
) -> StdResult<(Uint256, Decimal256)> {
    let reference_sum_snapshot = read_epoch_scale_sum(
        storage,
        &bid.collateral_token,
        bid.premium_slot,
        bid.epoch_snapshot,
        bid.scale_snapshot,
    )
    .unwrap_or_default();

    // reward = reward from first scale + reward from second scale (if any)
    let first_portion = reference_sum_snapshot - bid.sum_snapshot;
    let second_portion = if let Ok(second_scale_sum_snapshot) = read_epoch_scale_sum(
        storage,
        &bid.collateral_token,
        bid.premium_slot,
        bid.epoch_snapshot,
        bid.scale_snapshot + Uint128::from(1u128),
    ) {
        Decimal256(
            (second_scale_sum_snapshot.0 - reference_sum_snapshot.0) / U256::from(1_000_000_000u64),
        )
    } else {
        Decimal256::zero()
    };

    let liquidated_collateral_dec = Decimal256::from_uint256(bid.amount)
        * (first_portion + second_portion)
        / bid.product_snapshot;
    let liquidated_collateral = liquidated_collateral_dec * Uint256::one();
    // stacks the residue when converting to integer
    let residue_collateral =
        liquidated_collateral_dec - Decimal256::from_uint256(liquidated_collateral);

    Ok((liquidated_collateral, residue_collateral))
}

fn claim_col_residue(bid_pool: &mut BidPool) -> Uint256 {
    let claimable = bid_pool.residue_collateral * Uint256::one();
    if !claimable.is_zero() {
        bid_pool.residue_collateral =
            bid_pool.residue_collateral - Decimal256::from_uint256(claimable);
    }
    claimable
}

fn claim_bid_residue(bid_pool: &mut BidPool) -> Uint256 {
    let claimable = bid_pool.residue_bid * Uint256::one();
    if !claimable.is_zero() {
        bid_pool.residue_bid = bid_pool.residue_bid - Decimal256::from_uint256(claimable);
    }
    claimable
}
