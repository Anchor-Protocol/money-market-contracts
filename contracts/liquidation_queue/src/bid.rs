use crate::asserts::{assert_activate_status, assert_withdraw_amount};
use crate::state::{Bid, BidPool, CollateralInfo, Config, list_bid_pool, pop_bid_idx, pop_bid_pool_idx, read_active_bid_pool, read_available_bids, read_bid, read_bid_pool, read_bids_by_user, read_collateral_info, read_config, read_or_create_active_bid_pool, remove_bid, store_available_bids, store_bid, store_bid_pool};
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
        read_or_create_active_bid_pool(&mut deps.storage, &collateral_info, premium_slot)?;

    // create bid object
    let bid_idx: Uint128 = pop_bid_idx(&mut deps.storage)?;
    let mut bid = Bid {
        idx: bid_idx,
        owner: bidder_raw,
        premium_slot,
        collateral_token: collateral_token_raw.clone(),
        amount,
        ..Bid::default()
    };

    // if available bids is lower than bid_threshold, directly activate bid
    let available_bids: Uint256 = read_available_bids(&deps.storage, &collateral_token_raw)?;
    if available_bids < collateral_info.bid_threshold {
        let bid_pool_submited_to = process_bid_activation(&mut deps.storage, &mut bid, &mut bid_pool, amount)?;
        store_bid_pool(
            &mut deps.storage,
            &bid_pool_submited_to,
        )?;
        store_available_bids(
            &mut deps.storage,
            &collateral_token_raw,
            available_bids + amount,
        )?;
    } else {
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
    let available_bids: Uint256 = read_available_bids(&deps.storage, &collateral_token_raw)?;

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

    let mut total_activated_amount = Uint256::zero();
    for mut bid in bids.into_iter() {
        if bid.owner != sender_raw {
            return Err(StdError::unauthorized());
        }
        if bid.collateral_token != collateral_token_raw {
            return Err(StdError::generic_err(format!(
                "The bid_idx:{} corresponds to a different collateral",
                bid.idx
            )));
        }
        let mut bid_pool: BidPool =
            read_active_bid_pool(&deps.storage, &bid.collateral_token, bid.premium_slot)?;

        let amount_to_activate: Uint256 = bid.amount;
        total_activated_amount += amount_to_activate;

        // assert that the bid is inactive and wait period has expired
        assert_activate_status(&bid, &env)?;

        // update bid and bid pool, add new share and pool indexes to bid
        let bid_pool_submited_to = process_bid_activation(&mut deps.storage, &mut bid, &mut bid_pool, amount_to_activate)?;

        // save to storage
        store_bid(&mut deps.storage, bid.idx, &bid)?;

        store_bid_pool(
            &mut deps.storage,
            &bid_pool_submited_to,
        )?;
    }

    store_available_bids(
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

    if bid.owner != sender_raw {
        return Err(StdError::unauthorized());
    }

    let withdraw_amount: Uint256 = if bid.wait_end.is_some() {
        let waiting_withdraw_amount = assert_withdraw_amount(amount, bid.amount)?;
        if waiting_withdraw_amount == bid.amount {
            remove_bid(&mut deps.storage, bid.idx)?;
        } else {
            bid.amount = bid.amount - waiting_withdraw_amount;
            store_bid(&mut deps.storage, bid.idx, &bid)?;
        }

        waiting_withdraw_amount
    } else {
        let mut bid_pool: BidPool = read_bid_pool(&deps.storage, bid.bid_pool_idx)?;
        // update bid and obtain current withdrawable amount
        let withdrawable_amount: Uint256 =
            update_bid_and_calculate_withdrawable(&mut bid, &bid_pool);

        // check requested amount
        let active_withdraw_amount: Uint256 = assert_withdraw_amount(amount, withdrawable_amount)?;

        // remove share, and delete bid if there is nothing to claim
        let removed_share: Uint256 = calculate_removed_share(&bid_pool, active_withdraw_amount);

        // remove or update bid
        if active_withdraw_amount == withdrawable_amount
            && bid.pending_liquidated_collateral.is_zero()
        {
            remove_bid(&mut deps.storage, bid_idx)?;
        } else {
            bid.share = subtract_bid_share(&bid, removed_share);
            bid.amount = bid.amount - active_withdraw_amount;
            store_bid(&mut deps.storage, bid.idx, &bid)?;
        }

        // save bid pool
        bid_pool.total_bid_amount = bid_pool.total_bid_amount - active_withdraw_amount;
        bid_pool.total_share = bid_pool.total_share - removed_share;
        store_bid_pool(
            &mut deps.storage,
            &bid_pool,
        )?;

        active_withdraw_amount
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
    _liquidator: HumanAddr,
    repay_address: HumanAddr,
    fee_address: HumanAddr,
    collateral_token: HumanAddr,
    amount: Uint256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(&deps.storage, &collateral_token_raw)?;
    let available_bids: Uint256 = read_available_bids(&deps.storage, &collateral_token_raw)?;

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

    if amount * price.rate > available_bids {
        return Err(StdError::generic_err(
            "Not enough bids to execute this liquidation",
        ));
    }

    let mut remaining_collateral_to_liquidate = amount;
    let mut repay_amount = Uint256::zero();
    let mut filled: bool = false;
    'slots_iter: for slot in 0..collateral_info.max_slot {
        'inheritance_iter: loop {
            let mut bid_pool: BidPool = match read_active_bid_pool(&deps.storage, &collateral_token_raw, slot)
            {
                Ok(bid_pool) => bid_pool,
                Err(_) => continue 'slots_iter, // no bidders for this slot
            };
            if bid_pool.total_bid_amount.is_zero() {
                // check if ineritor exist and activate it
                if bid_pool.inheritor_pool_idx.is_some() {
                    list_bid_pool(&mut deps.storage, &collateral_token_raw, slot, bid_pool.inheritor_pool_idx.unwrap())?;
                    continue 'inheritance_iter;
                }
                // otherwise continue to next slot
                continue 'slots_iter;
            };
    
            let (pool_repay_amount, pool_liquidated_collateral) = execute_pool_liquidation(
                &mut bid_pool,
                remaining_collateral_to_liquidate,
                price.rate,
                &mut filled,
            );
            store_bid_pool(&mut deps.storage, &bid_pool)?;
    
            repay_amount += pool_repay_amount;
            if filled {
                break 'slots_iter;
            } else {
                remaining_collateral_to_liquidate =
                    remaining_collateral_to_liquidate - pool_liquidated_collateral;

                // if liquidation is not filled, check for inheritor and activate it
                if bid_pool.inheritor_pool_idx.is_some() {
                    list_bid_pool(&mut deps.storage, &collateral_token_raw, slot, bid_pool.inheritor_pool_idx.unwrap())?;
                } else {
                    break 'inheritance_iter;
                }
            }
        }
    }

    store_available_bids(
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
        if bid.owner != sender_raw {
            return Err(StdError::unauthorized());
        }

        let bid_pool: BidPool =
            read_bid_pool(&deps.storage, bid.bid_pool_idx)?;

        // calculate liquidated collateral
        let liquidated_collateral =
            bid.share * bid_pool.liquidation_index - bid.share * bid.liquidation_index;
        let spent_stable = bid.share * bid_pool.expense_index - bid.share * bid.expense_index;

        // get claimable amount
        claim_amount += bid.pending_liquidated_collateral + liquidated_collateral;
        let expense_amount = bid.spent + spent_stable;

        if spent_stable >= bid.amount {
            remove_bid(&mut deps.storage, bid.idx)?;
        } else {
            store_bid(
                &mut deps.storage,
                bid.idx,
                &Bid {
                    expense_index: bid_pool.expense_index,
                    liquidation_index: bid_pool.liquidation_index,
                    pending_liquidated_collateral: Uint256::zero(),
                    spent: expense_amount,
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
            log("action", "calim_liquidations"),
            log("collateral_token", collateral_token),
            log("collateral_amount", claim_amount),
        ],
        data: None,
    })
}

fn process_bid_activation<S: Storage>(storage: &mut S, bid: &mut Bid, bid_pool: &mut BidPool, amount: Uint256) -> StdResult<BidPool> {
    let mut active_bid_pool = if let Some(inheritor_pool_idx) = bid_pool.inheritor_pool_idx {
        read_bid_pool(storage, inheritor_pool_idx)?
    } else {
        bid_pool.clone()
    };

    let mut bid_share: Uint256 = if active_bid_pool.total_bid_amount.is_zero() {
        Uint256::from(1u64)
    } else {
        amount * Decimal256::from_uint256(active_bid_pool.total_share)
            / Decimal256::from_uint256(active_bid_pool.total_bid_amount)
    };

    // if share causes overflow, deprecate current pool and submit the bid to the inheritor pool
    if bid_share + active_bid_pool.total_share > Uint256::from(u128::MAX) {
        active_bid_pool = BidPool {
            idx: pop_bid_pool_idx(storage)?,
            liquidation_index: Decimal256::zero(),
            expense_index: Decimal256::zero(),
            total_bid_amount: Uint256::zero(),
            premium_rate: Decimal256::percent(bid.premium_slot as u64),
            total_share: Uint256::zero(),
            inheritor_pool_idx: None,
        };
        // update old pool
        bid_pool.inheritor_pool_idx = Some(active_bid_pool.idx);
        store_bid_pool(storage, bid_pool)?;

        bid_share = Uint256::from(1u64);
    }

    bid.share = bid_share;
    bid.liquidation_index = active_bid_pool.liquidation_index;
    bid.expense_index = active_bid_pool.expense_index;
    bid.bid_pool_idx = active_bid_pool.idx;
    bid.wait_end = None;

    active_bid_pool.total_bid_amount += amount;
    active_bid_pool.total_share += bid_share;

    Ok(active_bid_pool)
}

fn update_bid_and_calculate_withdrawable(bid: &mut Bid, bid_pool: &BidPool) -> Uint256 {
    // calculate liquidated collateral
    let liquidated_collateral =
        bid.share * bid_pool.liquidation_index - bid.share * bid.liquidation_index;
    let spent_stable = bid.share * bid_pool.expense_index - bid.share * bid.expense_index;

    // update indexes
    bid.expense_index = bid_pool.expense_index;
    bid.liquidation_index = bid_pool.liquidation_index;

    // update bid rewards/expenses
    bid.pending_liquidated_collateral += liquidated_collateral;
    bid.spent += spent_stable;

    if bid.spent < bid.amount {
        bid.amount - bid.spent
    } else {
        Uint256::zero()
    }
}

fn calculate_removed_share(bid_pool: &BidPool, withdraw_amount: Uint256) -> Uint256 {
    if bid_pool.total_bid_amount.is_zero() {
        withdraw_amount * Decimal256::from_uint256(bid_pool.total_share)
    } else {
        withdraw_amount * Decimal256::from_uint256(bid_pool.total_share)
            / Decimal256::from_uint256(bid_pool.total_bid_amount)
    }
}

fn subtract_bid_share(bid: &Bid, removed_share: Uint256) -> Uint256 {
    if removed_share > bid.share {
        Uint256::zero()
    } else {
        bid.share - removed_share
    }
}

fn execute_pool_liquidation(
    bid_pool: &mut BidPool,
    collateral_to_liquidate: Uint256,
    price: Decimal256,
    filled: &mut bool,
) -> (Uint256, Uint256) {
    let mut pool_collateral_to_liquidate = collateral_to_liquidate;
    let mut pool_required_stable =
        pool_collateral_to_liquidate * price * (Decimal256::one() - bid_pool.premium_rate);

    if pool_required_stable > bid_pool.total_bid_amount {
        pool_required_stable = bid_pool.total_bid_amount;
        pool_collateral_to_liquidate =
            pool_required_stable / ((Decimal256::one() - bid_pool.premium_rate) * price);
    } else {
        *filled = true;
    }

    let collateral_per_stable: Decimal256 = Decimal256::from_uint256(pool_collateral_to_liquidate)
        / Decimal256::from_uint256(bid_pool.total_share);
    let expense_per_stable: Decimal256 = Decimal256::from_uint256(pool_required_stable)
        / Decimal256::from_uint256(bid_pool.total_share);

    bid_pool.total_bid_amount = bid_pool.total_bid_amount - pool_required_stable;
    bid_pool.expense_index += expense_per_stable;
    bid_pool.liquidation_index += collateral_per_stable;

    (pool_required_stable, pool_collateral_to_liquidate)
}
