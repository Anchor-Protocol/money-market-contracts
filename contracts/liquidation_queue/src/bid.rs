use crate::asserts::{assert_activate_status, assert_withdraw_amount};
use crate::state::{
    pop_bid_idx, read_bid, read_bid_pool, read_bids_by_user, read_collateral_info, read_config,
    read_epoch_scale_sum, read_or_create_bid_pool, read_total_bids, remove_bid, store_bid,
    store_bid_pool, store_epoch_scale_sum, store_total_bids, Bid, BidPool, CollateralInfo, Config,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    log, to_binary, Api, BankMsg, CanonicalAddr, Coin, CosmosMsg, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20HandleMsg;
use moneymarket::oracle::PriceResponse;
use moneymarket::querier::{deduct_tax, query_price, TimeConstraints};

pub fn submit_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    premium_slot: u8,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let collateral_token_raw: CanonicalAddr = deps.api.canonical_address(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(&deps.storage, &collateral_token_raw)?;
    let bidder_raw = deps.api.canonical_address(&env.message.sender)?;

    let amount: Uint256 = Uint256::from(
        env.message
            .sent_funds
            .iter()
            .find(|c| c.denom == config.stable_denom)
            .map(|c| c.amount)
            .ok_or_else(|| {
                StdError::generic_err(format!(
                    "No {} assets have been provided",
                    config.stable_denom
                ))
            })?,
    );

    // read or create bid_pool, make sure slot is valid
    let mut bid_pool: BidPool =
        read_or_create_bid_pool(&mut deps.storage, &collateral_info, premium_slot)?;

    // create bid object
    let bid_idx: Uint128 = pop_bid_idx(&mut deps.storage)?;
    let mut bid = Bid {
        idx: bid_idx,
        bidder: bidder_raw,
        collateral_token: collateral_token_raw.clone(),
        product_snapshot: Decimal256::one(),
        amount,
        premium_slot: premium_slot,
        ..Bid::default()
    };

    // if available bids is lower than bid_threshold, directly activate bid
    let available_bids: Uint256 =
        read_total_bids(&deps.storage, &collateral_token_raw).unwrap_or_default();
    if available_bids < collateral_info.bid_threshold {
        // update bid and bid pool, add new share and pool indexes to bid
        process_bid_activation(&mut bid, &mut bid_pool, amount);

        // store bid_pool
        store_bid_pool(
            &mut deps.storage,
            &collateral_token_raw,
            premium_slot,
            &bid_pool,
        )?;

        // increase total bid amount
        store_total_bids(
            &mut deps.storage,
            &collateral_token_raw,
            available_bids + amount,
        )?;
    } else {
        // calculate wait_end from current time
        bid.wait_end = Some(env.block.time + config.waiting_period);
    };

    // save to storage
    store_bid(&mut deps.storage, bid_idx, &bid)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "submit_bid"),
            log("bid_idx", bid_idx),
            log("amount", amount),
        ],
        data: None,
    })
}

pub fn activate_bids<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    bids_idx: Option<Vec<Uint128>>,
) -> HandleResult {
    let sender_raw: CanonicalAddr = deps.api.canonical_address(&env.message.sender)?;
    let collateral_token_raw: CanonicalAddr = deps.api.canonical_address(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(&deps.storage, &collateral_token_raw)?;
    let available_bids: Uint256 =
        read_total_bids(&deps.storage, &collateral_token_raw).unwrap_or_default();

    let bids: Vec<Bid> = if let Some(bids_idx) = bids_idx {
        bids_idx
            .iter()
            .map(|idx| read_bid(&deps.storage, *idx))
            .collect::<StdResult<Vec<Bid>>>()?
    } else {
        read_bids_by_user(
            &deps.storage,
            &collateral_token_raw,
            &sender_raw,
            None,
            None,
        )?
        .into_iter()
        .filter(|b| b.wait_end.is_some())
        .collect::<Vec<Bid>>()
    };

    let mut total_activated_amount = Uint256::zero();
    for mut bid in bids.into_iter() {
        if bid.bidder != sender_raw {
            return Err(StdError::unauthorized());
        }
        if bid.collateral_token != collateral_token_raw {
            return Err(StdError::generic_err("Bid collateral token doesn't match"));
        }
        let mut bid_pool: BidPool =
            read_bid_pool(&deps.storage, &bid.collateral_token, bid.premium_slot)?;

        let amount_to_activate = bid.amount;

        // assert that the bid is inactive and wait period has expired
        assert_activate_status(&bid, &env, available_bids, collateral_info.bid_threshold)?;

        // update bid and bid pool, add new share and pool indexes to bid
        process_bid_activation(&mut bid, &mut bid_pool, amount_to_activate);

        // save to storage
        store_bid(&mut deps.storage, bid.idx, &bid)?;
        store_bid_pool(
            &mut deps.storage,
            &bid.collateral_token,
            bid.premium_slot,
            &bid_pool,
        )?;

        total_activated_amount += amount_to_activate;
    }

    store_total_bids(
        &mut deps.storage,
        &collateral_token_raw,
        available_bids + total_activated_amount,
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "activate_bids"),
            log("amount", total_activated_amount),
        ],
        data: None,
    })
}

pub fn retract_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bid_idx: Uint128,
    amount: Option<Uint256>,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let sender_raw: CanonicalAddr = deps.api.canonical_address(&env.message.sender)?;
    let mut bid: Bid = read_bid(&deps.storage, bid_idx)?;
    let collateral_token_raw: CanonicalAddr = bid.collateral_token.clone();

    if bid.bidder != sender_raw {
        return Err(StdError::unauthorized());
    }

    // check if bid is active or waiting
    let withdraw_amount: Uint256 = if bid.wait_end.is_some() {
        // waiting bid amount can be withdrawn without restriction
        let waiting_withdraw_amount = assert_withdraw_amount(amount, bid.amount)?;
        if waiting_withdraw_amount == bid.amount {
            remove_bid(&mut deps.storage, bid.idx)?;
        } else {
            bid.amount = bid.amount - waiting_withdraw_amount;
            store_bid(&mut deps.storage, bid.idx, &bid)?;
        }

        waiting_withdraw_amount
    } else {
        let available_bids: Uint256 = read_total_bids(&deps.storage, &collateral_token_raw)?;
        let mut bid_pool: BidPool =
            read_bid_pool(&deps.storage, &collateral_token_raw, bid.premium_slot)?;

        // calculate spent and reward until this moment
        let (withdrawable_amount, residue_bid) = calculate_remaining_bid(&bid, &bid_pool)?;
        let (liquidated_collateral, residue_collateral) =
            calculate_liquidated_collateral(&deps.storage, &bid)?;

        // accumulate pending reward to be claimed later
        bid.pending_liquidated_collateral += liquidated_collateral;

        // stack residues, will give it to next claimer if it becomes bigger than 1.0
        bid_pool.residue_collateral += residue_collateral;
        bid_pool.residue_bid += residue_bid;

        // check requested amount
        let withdraw_amount: Uint256 = assert_withdraw_amount(amount, withdrawable_amount)?;

        // remove or update bid
        if withdraw_amount == withdrawable_amount && bid.pending_liquidated_collateral.is_zero() {
            remove_bid(&mut deps.storage, bid_idx)?;
        } else {
            store_bid(
                &mut deps.storage,
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
            &mut deps.storage,
            &collateral_token_raw,
            bid.premium_slot,
            &bid_pool,
        )?;
        store_total_bids(
            &mut deps.storage,
            &collateral_token_raw,
            available_bids - withdraw_amount,
        )?;

        refund_amount
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    if !withdraw_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: env.message.sender.clone(),
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: config.stable_denom,
                    amount: withdraw_amount.into(),
                },
            )?],
        }));
    }

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "retract_bid"),
            log("bid_idx", bid_idx),
            log("amount", withdraw_amount),
        ],
        data: None,
    })
}

pub fn execute_liquidation<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    repay_address: HumanAddr,
    fee_address: HumanAddr,
    collateral_token: HumanAddr,
    amount: Uint256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(&deps.storage, &collateral_token_raw)?;
    let available_bids: Uint256 = read_total_bids(&deps.storage, &collateral_token_raw)?;

    let oracle_contract = deps.api.human_address(&config.oracle_contract)?;
    let price: PriceResponse = query_price(
        &deps,
        &oracle_contract,
        collateral_token.to_string(),
        config.stable_denom.clone(),
        Some(TimeConstraints {
            block_time: env.block.time,
            valid_timeframe: config.price_timeframe,
        }),
    )?;

    let mut remaining_collateral_to_liquidate = amount;
    let mut repay_amount = Uint256::zero();
    let mut filled: bool = false;
    for slot in 0..collateral_info.max_slot + 1 {
        let mut bid_pool: BidPool = match read_bid_pool(&deps.storage, &collateral_token_raw, slot)
        {
            Ok(bid_pool) => bid_pool,
            Err(_) => continue,
        };
        if bid_pool.total_bid_amount.is_zero() {
            continue;
        };

        let (pool_repay_amount, pool_liquidated_collateral) = execute_pool_liquidation(
            &mut deps.storage,
            &mut bid_pool,
            &collateral_token_raw,
            slot,
            remaining_collateral_to_liquidate,
            price.rate,
            &mut filled,
        )?;

        store_bid_pool(&mut deps.storage, &collateral_token_raw, slot, &bid_pool)?;

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
        &mut deps.storage,
        &collateral_token_raw,
        available_bids - repay_amount,
    )?;

    let bid_fee = repay_amount * config.bid_fee;
    let repay_amount = repay_amount - bid_fee;

    let mut messages: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: repay_address,
        amount: vec![deduct_tax(
            &deps,
            Coin {
                denom: config.stable_denom.clone(),
                amount: repay_amount.into(),
            },
        )?],
    })];

    if !bid_fee.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: fee_address,
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: config.stable_denom.clone(),
                    amount: bid_fee.into(),
                },
            )?],
        }));
    }

    Ok(HandleResponse {
        messages,
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

pub fn claim_liquidations<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    bids_idx: Option<Vec<Uint128>>,
) -> HandleResult {
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;

    let bids: Vec<Bid> = if let Some(bids_idx) = bids_idx {
        bids_idx
            .iter()
            .map(|idx| read_bid(&deps.storage, *idx))
            .collect::<StdResult<Vec<Bid>>>()?
    } else {
        read_bids_by_user(
            &deps.storage,
            &collateral_token_raw,
            &sender_raw,
            None,
            None,
        )?
    };

    let mut claim_amount = Uint256::zero();
    for bid in bids.into_iter() {
        if bid.bidder != sender_raw {
            return Err(StdError::unauthorized());
        }
        if bid.collateral_token != collateral_token_raw {
            return Err(StdError::generic_err("Bid collateral token doesn't match"));
        }

        let mut bid_pool: BidPool =
            read_bid_pool(&deps.storage, &bid.collateral_token, bid.premium_slot)?;

        // calculate remaining bid amount
        let (remaining_bid, residue_bid) = calculate_remaining_bid(&bid, &bid_pool)?;

        // calculate liquidated collateral
        let (liquidated_collateral, residue_collateral) =
            calculate_liquidated_collateral(&deps.storage, &bid)?;

        // keep residues
        bid_pool.residue_collateral += residue_collateral;
        bid_pool.residue_bid += residue_bid;

        // get claimable amount
        claim_amount += bid.pending_liquidated_collateral
            + liquidated_collateral
            + claim_col_residue(&mut bid_pool);

        // store bid_pool to update residue
        store_bid_pool(
            &mut deps.storage,
            &collateral_token_raw,
            bid.premium_slot,
            &bid_pool,
        )?;

        // check if bid has been consumed, include 1 for rounding
        if remaining_bid <= Uint256::one() {
            remove_bid(&mut deps.storage, bid.idx)?;
        } else {
            store_bid(
                &mut deps.storage,
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

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral_token.clone(),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: env.message.sender,
                amount: claim_amount.into(),
            })?,
        })],
        log: vec![
            log("action", "claim_liquidations"),
            log("collateral_token", collateral_token),
            log("collateral_amount", claim_amount),
        ],
        data: None,
    })
}

fn process_bid_activation(bid: &mut Bid, bid_pool: &mut BidPool, amount: Uint256) {
    bid.product_snapshot = bid_pool.product_snapshot;
    bid.sum_snapshot = bid_pool.sum_snapshot;
    bid.wait_end = None;
    bid.scale_snapshot = bid_pool.current_scale;
    bid.epoch_snapshot = bid_pool.current_epoch;

    bid_pool.total_bid_amount += amount;
}

fn execute_pool_liquidation<S: Storage>(
    storage: &mut S,
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
        &collateral_token,
        premium_slot,
        bid_pool.current_epoch,
        bid_pool.current_scale,
        bid_pool.sum_snapshot,
    )?;

    // Update product
    // Check if the pool is emptied, if it is, reset (P = 1, S = 0)
    if expense_per_bid == Decimal256::one() {
        bid_pool.sum_snapshot = Decimal256::zero();
        bid_pool.product_snapshot = Decimal256::one();
        bid_pool.current_scale = Uint128::zero();

        bid_pool.current_epoch += Uint128(1);
    } else {
        // 1 - Q / D
        let product = Decimal256::one() - expense_per_bid;

        // check if scale needs to be increased (in case product truncates to zero)
        // seems hard to test this, TODO: include this to be tested
        let new_product = bid_pool.product_snapshot * product;
        bid_pool.product_snapshot = if new_product.is_zero() {
            bid_pool.current_scale += Uint128::from(1u128);
            Decimal256(bid_pool.product_snapshot.0 * Decimal256::DECIMAL_FRACTIONAL) * product
        } else {
            new_product
        };
    }

    Ok((pool_required_stable, pool_collateral_to_liquidate))
}

pub(crate) fn calculate_remaining_bid(
    bid: &Bid,
    bid_pool: &BidPool,
) -> StdResult<(Uint256, Decimal256)> {
    let scale_diff: Uint128 = (bid_pool.current_scale - bid.scale_snapshot)?;
    let epoch_diff: Uint128 = (bid_pool.current_epoch - bid.epoch_snapshot)?;

    let remaining_bid_dec: Decimal256 = if !epoch_diff.is_zero() {
        // pool was emptied, return 0
        Decimal256::zero()
    } else if scale_diff.is_zero() {
        Decimal256::from_uint256(bid.amount) * (bid_pool.product_snapshot / bid.product_snapshot)
    } else if scale_diff == Uint128(1) {
        // product has been scaled
        let scaled_remaining_bid = Decimal256::from_uint256(bid.amount)
            * (bid_pool.product_snapshot / bid.product_snapshot);
        Decimal256(scaled_remaining_bid.0 / Decimal256::DECIMAL_FRACTIONAL)
    } else {
        Decimal256::zero()
    };

    let remaining_bid = remaining_bid_dec * Uint256::one();
    let bid_residue = remaining_bid_dec - Decimal256::from_uint256(remaining_bid);

    Ok((remaining_bid, bid_residue))
}

pub(crate) fn calculate_liquidated_collateral<S: Storage>(
    storage: &S,
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
        bid.scale_snapshot + Uint128(1),
    ) {
        Decimal256(second_scale_sum_snapshot.0 / Decimal256::DECIMAL_FRACTIONAL)
    } else {
        Decimal256::zero()
    };

    let liquidated_collateral_dec = Decimal256::from_uint256(bid.amount)
        * ((first_portion + second_portion) / bid.product_snapshot);
    let liquidated_collateral = liquidated_collateral_dec * Uint256::one();
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
