use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, Order, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use moneymarket::liquidation::BidResponse;

static KEY_CONFIG: &[u8] = b"config";

static PREFIX_BID: &[u8] = b"bid";
static PREFIX_BID_BY_USER: &[u8] = b"bid_by_user";
static PREFIX_BID_BY_COLLATERAL: &[u8] = b"bid_by_collateral";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub stable_denom: String,
    pub safe_ratio: Decimal256,
    pub bid_fee: Decimal256,
    pub max_premium_rate: Decimal256,
    pub liquidation_threshold: Uint256,
    pub price_timeframe: u64,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub amount: Uint256,
    pub premium_rate: Decimal256,
}

pub fn store_bid(
    storage: &mut dyn Storage,
    bidder: &CanonicalAddr,
    collateral_token: &CanonicalAddr,
    bid: Bid,
) -> StdResult<()> {
    let mut bid_bucket: Bucket<Bid> = Bucket::new(storage, PREFIX_BID);
    bid_bucket.save(
        &[bidder.as_slice(), collateral_token.as_slice()].concat(),
        &bid,
    )?;

    let mut bid_user_index: Bucket<bool> =
        Bucket::multilevel(storage, &[PREFIX_BID_BY_USER, bidder.as_slice()]);
    bid_user_index.save(collateral_token.as_slice(), &true)?;

    let mut bid_collateral_index: Bucket<bool> = Bucket::multilevel(
        storage,
        &[PREFIX_BID_BY_COLLATERAL, collateral_token.as_slice()],
    );
    bid_collateral_index.save(bidder.as_slice(), &true)?;

    Ok(())
}

pub fn remove_bid(
    storage: &mut dyn Storage,
    bidder: &CanonicalAddr,
    collateral_token: &CanonicalAddr,
) {
    let mut bid_bucket: Bucket<Bid> = Bucket::new(storage, PREFIX_BID);
    bid_bucket.remove(&[bidder.as_slice(), collateral_token.as_slice()].concat());

    let mut bid_user_index: Bucket<bool> =
        Bucket::multilevel(storage, &[PREFIX_BID_BY_USER, bidder.as_slice()]);
    bid_user_index.remove(collateral_token.as_slice());

    let mut bid_collateral_index: Bucket<bool> = Bucket::multilevel(
        storage,
        &[PREFIX_BID_BY_COLLATERAL, collateral_token.as_slice()],
    );
    bid_collateral_index.remove(bidder.as_slice());
}

pub fn read_bid<'a>(
    storage: &'a dyn Storage,
    bidder: &CanonicalAddr,
    collateral_token: &CanonicalAddr,
) -> StdResult<Bid> {
    let bid_bucket: ReadonlyBucket<'a, Bid> = ReadonlyBucket::new(storage, PREFIX_BID);

    bid_bucket
        .load(&[bidder.as_slice(), collateral_token.as_slice()].concat())
        .map_err(|_| StdError::generic_err("No bids with the specified information exist"))
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_bids_by_collateral(
    deps: Deps,
    collateral_token: &CanonicalAddr,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<BidResponse>> {
    let bid_bucket: ReadonlyBucket<bool> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_BID_BY_COLLATERAL, collateral_token.as_slice()],
    );

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    bid_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, _) = elem?;
            let bidder = CanonicalAddr::from(k);
            let bid = read_bid(deps.storage, &bidder, collateral_token)?;

            let bidder = deps.api.addr_humanize(&bidder)?.to_string();
            let collateral_token = deps.api.addr_humanize(collateral_token)?.to_string();
            let amount = bid.amount;
            let premium_rate = bid.premium_rate;

            Ok(BidResponse {
                collateral_token,
                bidder,
                amount,
                premium_rate,
            })
        })
        .collect()
}

pub fn read_bids_by_user(
    deps: Deps,
    bidder: &CanonicalAddr,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<BidResponse>> {
    let bid_bucket: ReadonlyBucket<bool> =
        ReadonlyBucket::multilevel(deps.storage, &[PREFIX_BID_BY_USER, bidder.as_slice()]);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    bid_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, _) = elem?;
            let collateral_token = CanonicalAddr::from(k);
            let bid = read_bid(deps.storage, bidder, &collateral_token)?;

            let collateral_token = deps.api.addr_humanize(&collateral_token)?.to_string();
            let bidder = deps.api.addr_humanize(bidder)?.to_string();
            let amount = bid.amount;
            let premium_rate = bid.premium_rate;

            Ok(BidResponse {
                collateral_token,
                bidder,
                amount,
                premium_rate,
            })
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<CanonicalAddr>) -> Option<Vec<u8>> {
    start_after.map(|addr| {
        let mut v = addr.as_slice().to_vec();
        v.push(1);
        v
    })
}
