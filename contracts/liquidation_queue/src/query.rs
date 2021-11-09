use crate::bid::{calculate_liquidated_collateral, calculate_remaining_bid};
use crate::querier::query_collateral_whitelist_info;
use crate::state::{
    read_bid, read_bid_pool, read_bid_pools, read_bids_by_user, read_collateral_info, read_config,
    read_total_bids, Bid, BidPool, CollateralInfo, Config,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, StdResult, Uint128};
use moneymarket::liquidation_queue::{
    BidPoolResponse, BidPoolsResponse, BidResponse, BidsResponse, CollateralInfoResponse,
    ConfigResponse, LiquidationAmountResponse,
};
use moneymarket::querier::query_tax_rate_and_cap;
use moneymarket::tokens::TokensHuman;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        oracle_contract: deps.api.addr_humanize(&config.oracle_contract)?.to_string(),
        stable_denom: config.stable_denom,
        safe_ratio: config.safe_ratio,
        bid_fee: config.bid_fee,
        liquidator_fee: config.liquidator_fee,
        liquidation_threshold: config.liquidation_threshold,
        price_timeframe: config.price_timeframe,
        waiting_period: config.waiting_period,
        overseer: deps.api.addr_humanize(&config.overseer)?.to_string(),
    };

    Ok(resp)
}

/// The amount of collateral to be liquidated depends on the status of the bid pools
/// for each collateral. To find out how much collateral should be liquidated
/// we find the intersaction between f(x) and g(x); where x = liquidated collateral,
/// f(x) determines liquidation amount at which the safe ratio is satisfied, and g(x) gives
/// the repay amount based on the collateral being liquidated, which takes into account the
/// available bids at different premium rates
pub fn query_liquidation_amount(
    deps: Deps,
    borrow_amount: Uint256,
    borrow_limit: Uint256,
    collaterals: TokensHuman,
    collateral_prices: Vec<Decimal256>,
) -> StdResult<LiquidationAmountResponse> {
    let config: Config = read_config(deps.storage)?;
    let overseer: String = deps.api.addr_humanize(&config.overseer)?.to_string();

    // Safely collateralized check
    if borrow_amount <= borrow_limit {
        return Ok(LiquidationAmountResponse {
            collaterals: vec![],
        });
    }

    // calculate value of all collaterals and weights
    let (collaterals_value, total_weight, collateral_weights, max_ltvs) =
        compute_collateral_weights(deps, overseer, &collaterals, &collateral_prices)?;

    // check partial liquidation condition
    let safe_ratio = if collaterals_value <= config.liquidation_threshold {
        Decimal256::zero()
    } else {
        config.safe_ratio
    };

    // check tax cap
    let (mut tax_rate, tax_cap) = query_tax_rate_and_cap(deps, config.stable_denom)?;
    let mut tax_cap_adj = tax_cap;
    if borrow_amount * tax_rate > tax_cap_adj {
        tax_rate = Decimal256::zero()
    } else {
        tax_cap_adj = Uint256::from(1u128)
    }

    let base_fee_deductor = (Decimal256::one() - config.bid_fee)
        * (Decimal256::one() - config.liquidator_fee)
        * (Decimal256::one() - tax_rate);

    let mut result: Vec<(String, Uint256)> = vec![];
    for (i, collateral) in collaterals.iter().enumerate() {
        let (price, weight, max_ltv) = (collateral_prices[i], collateral_weights[i], max_ltvs[i]);

        let collateral_token_raw = deps.api.addr_canonicalize(&collateral.0)?;
        let collateral_info = read_collateral_info(deps.storage, &collateral_token_raw)?;

        // calculate borrow amount and limit portion
        let position_portion =
            Decimal256::from_uint256(weight) / Decimal256::from_uint256(total_weight);
        let collateral_borrow_amount = borrow_amount * position_portion;
        let collateral_borrow_limit = borrow_limit * position_portion;

        // iterate bid pools until safe ratio condition is met (intersection f(x) and g(x))
        let mut x = Uint256::zero();
        let mut g_x = Uint256::zero();
        let mut intersected = false;
        for slot in 0..collateral_info.max_slot + 1 {
            let (slot_available_bids, premium_rate) =
                match read_bid_pool(deps.storage, &collateral_token_raw, slot) {
                    Ok(bid_pool) => (bid_pool.total_bid_amount, bid_pool.premium_rate),
                    Err(_) => continue,
                };
            if slot_available_bids.is_zero() {
                continue;
            };

            let prev_x = x;
            let prev_g_x = g_x;

            let discounted_price = price * (Decimal256::one() - premium_rate) * base_fee_deductor;
            x += slot_available_bids / discounted_price;

            let safe_borrow = safe_ratio * collateral_borrow_limit;
            let f_x = ((safe_ratio * max_ltv * price) * x) + collateral_borrow_amount - safe_borrow
                + tax_cap_adj;

            g_x += slot_available_bids;

            if g_x > f_x {
                let nominator = collateral_borrow_amount - safe_borrow
                    + tax_cap_adj
                    + (discounted_price * prev_x)
                    - prev_g_x;
                let denominator = price
                    * (((Decimal256::one() - premium_rate) * base_fee_deductor)
                        - (safe_ratio * max_ltv));

                let liquidation_amount = (nominator / denominator) + Uint256::one(); // round up

                result.push((
                    collateral.0.to_string(),
                    liquidation_amount.min(collateral.1),
                ));
                intersected = true;
                break;
            }
        }
        // Intersection is not reached in two situations:
        //      1. Not enough bids. Should try to liquidate as much as possible
        //      2. Not enouugh collateral. Also liquidate all collateral
        if !intersected {
            result.push((collateral.0.to_string(), x)); // liquidate all collateral possible
        }
    }

    Ok(LiquidationAmountResponse {
        collaterals: result,
    })
}

/// The portion of collateral that liquidated from the available set is calculated
/// based on weight = min(collateral_value, available_bids) / max_ltv
#[allow(clippy::ptr_arg)]
fn compute_collateral_weights(
    deps: Deps,
    overseer: String,
    collaterals: &TokensHuman,
    collateral_prices: &Vec<Decimal256>,
) -> StdResult<(Uint256, Uint256, Vec<Uint256>, Vec<Decimal256>)> {
    let mut collaterals_value = Uint256::zero();
    let mut total_weight = Uint256::zero();
    let mut collateral_weights: Vec<Uint256> = vec![];
    let mut max_ltvs: Vec<Decimal256> = vec![];

    for (collateral, price) in collaterals.iter().zip(collateral_prices.iter()) {
        let collateral_available_bids =
            read_total_bids(deps.storage, &deps.api.addr_canonicalize(&collateral.0)?)
                .unwrap_or_default();
        let max_ltv = query_collateral_whitelist_info(
            &deps.querier,
            overseer.to_string(),
            collateral.0.to_string(),
        )?
        .max_ltv;

        let collateral_value = collateral.1 * *price;
        let weigth = collateral_value.min(collateral_available_bids) / max_ltv;

        total_weight += weigth;
        collaterals_value += collateral_value;
        collateral_weights.push(weigth);
        max_ltvs.push(max_ltv);
    }

    Ok((
        collaterals_value,
        total_weight,
        collateral_weights,
        max_ltvs,
    ))
}

pub fn query_bid(deps: Deps, bid_idx: Uint128) -> StdResult<BidResponse> {
    let bid: Bid = read_bid(deps.storage, bid_idx)?;
    let bid_pool: BidPool = read_bid_pool(deps.storage, &bid.collateral_token, bid.premium_slot)?;

    let (bid_amount, bid_pending_liquidated_collateral) = if bid.wait_end.is_some() {
        (bid.amount, bid.pending_liquidated_collateral)
    } else {
        // calculate remaining bid amount
        let (remaining_bid, _) = calculate_remaining_bid(&bid, &bid_pool)?;

        // calculate liquidated collateral
        let (liquidated_collateral, _) = calculate_liquidated_collateral(deps.storage, &bid)?;

        (
            remaining_bid,
            bid.pending_liquidated_collateral + liquidated_collateral,
        )
    };

    Ok(BidResponse {
        idx: bid.idx,
        collateral_token: deps.api.addr_humanize(&bid.collateral_token)?.to_string(),
        bidder: deps.api.addr_humanize(&bid.bidder)?.to_string(),
        amount: bid_amount,
        premium_slot: bid.premium_slot,
        pending_liquidated_collateral: bid_pending_liquidated_collateral,
        product_snapshot: bid.product_snapshot,
        sum_snapshot: bid.sum_snapshot,
        wait_end: bid.wait_end,
        epoch_snapshot: bid.epoch_snapshot,
        scale_snapshot: bid.scale_snapshot,
    })
}

pub fn query_bids_by_user(
    deps: Deps,
    collateral_token: String,
    bidder: String,
    start_after: Option<Uint128>,
    limit: Option<u8>,
) -> StdResult<BidsResponse> {
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;
    let bidder_raw = deps.api.addr_canonicalize(&bidder)?;

    let bids: Vec<BidResponse> = read_bids_by_user(
        deps.storage,
        &collateral_token_raw,
        &bidder_raw,
        start_after,
        limit,
    )?
    .iter()
    .map(|bid| {
        let bid_pool: BidPool =
            read_bid_pool(deps.storage, &bid.collateral_token, bid.premium_slot)?;
        let (bid_amount, bid_pending_liquidated_collateral) = if bid.wait_end.is_some() {
            (bid.amount, bid.pending_liquidated_collateral)
        } else {
            // calculate remaining bid amount
            let (remaining_bid, _) = calculate_remaining_bid(bid, &bid_pool)?;

            // calculate liquidated collateral
            let (liquidated_collateral, _) = calculate_liquidated_collateral(deps.storage, bid)?;

            (
                remaining_bid,
                bid.pending_liquidated_collateral + liquidated_collateral,
            )
        };
        let res = BidResponse {
            idx: bid.idx,
            collateral_token: deps.api.addr_humanize(&bid.collateral_token)?.to_string(),
            bidder: deps.api.addr_humanize(&bid.bidder)?.to_string(),
            amount: bid_amount,
            premium_slot: bid.premium_slot,
            pending_liquidated_collateral: bid_pending_liquidated_collateral,
            product_snapshot: bid.product_snapshot,
            sum_snapshot: bid.sum_snapshot,
            wait_end: bid.wait_end,
            epoch_snapshot: bid.epoch_snapshot,
            scale_snapshot: bid.scale_snapshot,
        };
        Ok(res)
    })
    .collect::<StdResult<Vec<BidResponse>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_bid_pool(
    deps: Deps,
    collateral_token: String,
    bid_slot: u8,
) -> StdResult<BidPoolResponse> {
    let collateral_token_raw: CanonicalAddr = deps.api.addr_canonicalize(&collateral_token)?;
    let bid_pool: BidPool = read_bid_pool(deps.storage, &collateral_token_raw, bid_slot)?;

    Ok(BidPoolResponse {
        sum_snapshot: bid_pool.sum_snapshot,
        product_snapshot: bid_pool.product_snapshot,
        total_bid_amount: bid_pool.total_bid_amount,
        premium_rate: bid_pool.premium_rate,
        current_epoch: bid_pool.current_epoch,
        current_scale: bid_pool.current_scale,
    })
}

pub fn query_bid_pools(
    deps: Deps,
    collateral_token: String,
    start_after: Option<u8>,
    limit: Option<u8>,
) -> StdResult<BidPoolsResponse> {
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;

    let bid_pools: Vec<BidPoolResponse> =
        read_bid_pools(deps.storage, &collateral_token_raw, start_after, limit)?
            .iter()
            .map(|bid_pool| BidPoolResponse {
                sum_snapshot: bid_pool.sum_snapshot,
                product_snapshot: bid_pool.product_snapshot,
                total_bid_amount: bid_pool.total_bid_amount,
                premium_rate: bid_pool.premium_rate,
                current_epoch: bid_pool.current_epoch,
                current_scale: bid_pool.current_scale,
            })
            .collect();

    Ok(BidPoolsResponse { bid_pools })
}

pub fn query_collateral_info(
    deps: Deps,
    collateral_token: String,
) -> StdResult<CollateralInfoResponse> {
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;
    let collateral_info: CollateralInfo =
        read_collateral_info(deps.storage, &collateral_token_raw)?;

    Ok(CollateralInfoResponse {
        collateral_token: deps.api.addr_humanize(&collateral_token_raw)?.to_string(),
        bid_threshold: collateral_info.bid_threshold,
        max_slot: collateral_info.max_slot,
        premium_rate_per_slot: collateral_info.premium_rate_per_slot,
    })
}
