use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Order, ReadonlyStorage, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use std::convert::TryInto;

static KEY_CONFIG: &[u8] = b"config";
static KEY_BID_IDX: &[u8] = b"bid_idx";
static KEY_BID_POOL_IDX: &[u8] = b"bid_pool_idx";

static PREFIX_AVAILABLE_BIDS: &[u8] = b"available_bids";
static PREFIX_BID: &[u8] = b"bid";
static PREFIX_BID_BY_USER: &[u8] = b"bid_by_user";
static PREFIX_BID_POOL: &[u8] = b"bid_pool";
static PREFIX_ACTIVE_BID_POOL_BY_COLLATERAL: &[u8] = b"bid_pool_by_col";
static PREFIX_COLLATERAL_INFO: &[u8] = b"col_info";

const MAX_LIMIT: u8 = 30;
const DEFAULT_LIMIT: u8 = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub stable_denom: String,
    pub safe_ratio: Decimal256,
    pub bid_fee: Decimal256,
    pub liquidation_threshold: Uint256,
    pub price_timeframe: u64,
    pub waiting_period: u64,
}

pub fn store_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config<S: ReadonlyStorage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn pop_bid_idx<S: Storage>(storage: &mut S) -> StdResult<Uint128> {
    let mut idx_store = singleton(storage, KEY_BID_IDX);
    let last_idx: Uint128 = idx_store.load().unwrap_or(Uint128::from(1u128));
    idx_store.save(&(last_idx + Uint128::from(1u128)))?;
    Ok(last_idx)
}

pub fn pop_bid_pool_idx<S: Storage>(storage: &mut S) -> StdResult<Uint128> {
    let mut idx_store = singleton(storage, KEY_BID_POOL_IDX);
    let last_idx: Uint128 = idx_store.load().unwrap_or(Uint128::from(1u128));
    idx_store.save(&(last_idx + Uint128::from(1u128)))?;
    Ok(last_idx)
}

pub fn store_available_bids<S: Storage>(
    storage: &mut S,
    collateral_token: &CanonicalAddr,
    available_bids: Uint256,
) -> StdResult<()> {
    let mut available_bids_bucket: Bucket<S, Uint256> = Bucket::new(PREFIX_AVAILABLE_BIDS, storage);
    available_bids_bucket.save(&collateral_token.as_slice(), &available_bids)?;

    Ok(())
}

pub fn read_available_bids<S: Storage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
) -> StdResult<Uint256> {
    let available_bids_bucket: ReadonlyBucket<S, Uint256> =
        ReadonlyBucket::new(PREFIX_AVAILABLE_BIDS, storage);
    available_bids_bucket.load(&collateral_token.as_slice())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollateralInfo {
    pub collateral_token: CanonicalAddr,
    pub bid_threshold: Uint256,
    pub max_slot: u8,
}

pub fn store_collateral_info<S: Storage>(
    storage: &mut S,
    collateral_token: &CanonicalAddr,
    collateral_info: &CollateralInfo,
) -> StdResult<()> {
    let mut collateral_info_bucket: Bucket<S, CollateralInfo> =
        Bucket::new(PREFIX_COLLATERAL_INFO, storage);
    collateral_info_bucket.save(&collateral_token.as_slice(), &collateral_info)?;

    Ok(())
}

pub fn read_collateral_info<S: Storage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
) -> StdResult<CollateralInfo> {
    let collateral_info_bucket: ReadonlyBucket<S, CollateralInfo> =
        ReadonlyBucket::new(PREFIX_COLLATERAL_INFO, storage);
    collateral_info_bucket
        .load(&collateral_token.as_slice())
        .map_err(|_| StdError::generic_err("Collateral is not whitelisted"))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidPool {
    pub idx: Uint128,
    pub liquidation_index: Decimal256,
    pub expense_index: Decimal256,
    pub total_share: Uint256,
    pub total_bid_amount: Uint256,
    pub premium_rate: Decimal256,
    pub inheritor_pool_idx: Option<Uint128>,
}

pub fn store_bid_pool<S: Storage>(
    storage: &mut S,
    bid_pool: &BidPool,
) -> StdResult<()> {
    let mut bid_pool_bucket: Bucket<S, BidPool> = Bucket::new(PREFIX_BID_POOL, storage);
    bid_pool_bucket.save(&bid_pool.idx.u128().to_be_bytes(), &bid_pool)?;

    Ok(())
}

pub fn list_bid_pool<S: Storage>(
    storage: &mut S,
    collateral_token: &CanonicalAddr,
    premium_bid_slot: u8,
    bid_pool_idx: Uint128,
) -> StdResult<()> {
    let mut active_bid_pool_bucket: Bucket<S, Uint128> = Bucket::multilevel(
        &[PREFIX_ACTIVE_BID_POOL_BY_COLLATERAL, collateral_token.as_slice()],
        storage,
    );
    active_bid_pool_bucket.save(&premium_bid_slot.to_be_bytes(), &bid_pool_idx)
}

pub fn read_bid_pool<S: Storage>(
    storage: &S,
    bid_pool_idx: Uint128,
) -> StdResult<BidPool> {
    let bid_pool_bucket: ReadonlyBucket<S, BidPool> = ReadonlyBucket::new(PREFIX_BID_POOL, storage);
    bid_pool_bucket
        .load(&bid_pool_idx.u128().to_be_bytes())
        .map_err(|_| StdError::generic_err("Bid pool not found"))
}

pub fn read_or_create_active_bid_pool<S: Storage>(
    storage: &mut S,
    collateral_info: &CollateralInfo,
    premium_slot: u8,
) -> StdResult<BidPool> {
    let active_bid_pool_bucket: ReadonlyBucket<S, Uint128> = ReadonlyBucket::multilevel(
        &[PREFIX_ACTIVE_BID_POOL_BY_COLLATERAL, collateral_info.collateral_token.as_slice()],
        storage,
    );

    match active_bid_pool_bucket.load(&premium_slot.to_be_bytes()) {
        Ok(bid_pool_idx) => read_bid_pool(storage, bid_pool_idx),
        Err(_) => {
            if (0..collateral_info.max_slot).contains(&premium_slot) {
                let bid_pool = BidPool {
                    idx: pop_bid_pool_idx(storage)?,
                    liquidation_index: Decimal256::zero(),
                    expense_index: Decimal256::zero(),
                    total_bid_amount: Uint256::zero(),
                    premium_rate: Decimal256::percent(premium_slot as u64),
                    total_share: Uint256::zero(),
                    inheritor_pool_idx: None,
                };
                store_bid_pool(storage, &bid_pool)?;
                list_bid_pool(
                    storage,
                    &collateral_info.collateral_token,
                    premium_slot,
                    bid_pool.idx,
                )?;
                Ok(bid_pool)
            } else {
                return Err(StdError::generic_err("Invalid premium slot"));
            }
        }
    }
}

pub fn read_active_bid_pool<S: Storage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
    premium_slot: u8,
) -> StdResult<BidPool> {
    let active_bid_pool_bucket: ReadonlyBucket<S, Uint128> = ReadonlyBucket::multilevel(
        &[PREFIX_ACTIVE_BID_POOL_BY_COLLATERAL, collateral_token.as_slice()],
        storage,
    );
    let bid_pool_idx: Uint128 = active_bid_pool_bucket.load(&premium_slot.to_be_bytes())?;
    read_bid_pool(storage, bid_pool_idx)
}

pub fn read_active_bid_pools<S: Storage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
    start_after: Option<u8>,
    limit: Option<u8>,
) -> StdResult<Vec<BidPool>> {
    let active_bid_pool_bucket: ReadonlyBucket<S, Uint128> = ReadonlyBucket::multilevel(
        &[PREFIX_ACTIVE_BID_POOL_BY_COLLATERAL, collateral_token.as_slice()],
        storage,
    );

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    active_bid_pool_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (_, pool_idx) = elem?;
            read_bid_pool(storage, pool_idx)
        })
        .collect()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Bid {
    pub idx: Uint128,
    pub bid_pool_idx: Uint128,
    pub owner: CanonicalAddr,
    pub amount: Uint256,
    pub share: Uint256,
    pub collateral_token: CanonicalAddr,
    pub premium_slot: u8,
    pub liquidation_index: Decimal256,
    pub expense_index: Decimal256,
    pub pending_liquidated_collateral: Uint256,
    pub spent: Uint256,
    pub wait_end: Option<u64>,
}

pub fn store_bid<S: Storage>(storage: &mut S, bid_idx: Uint128, bid: &Bid) -> StdResult<()> {
    let mut bid_bucket: Bucket<S, Bid> = Bucket::new(PREFIX_BID, storage);
    bid_bucket.save(&bid_idx.u128().to_be_bytes(), &bid)?;

    let mut bid_indexer_by_user: Bucket<S, bool> = Bucket::multilevel(
        &[
            PREFIX_BID_BY_USER,
            bid.collateral_token.as_slice(),
            bid.owner.as_slice(),
        ],
        storage,
    );
    bid_indexer_by_user.save(&bid_idx.u128().to_be_bytes(), &true)?;

    Ok(())
}

pub fn remove_bid<S: Storage>(storage: &mut S, bid_idx: Uint128) -> StdResult<()> {
    let bid: Bid = read_bid(storage, bid_idx)?;
    let mut bid_bucket: Bucket<S, Bid> = Bucket::new(PREFIX_BID, storage);
    bid_bucket.remove(&bid_idx.u128().to_be_bytes());

    // remove indexer
    let mut bid_indexer_by_user: Bucket<S, bool> = Bucket::multilevel(
        &[
            PREFIX_BID_BY_USER,
            bid.collateral_token.as_slice(),
            bid.owner.as_slice(),
        ],
        storage,
    );
    bid_indexer_by_user.remove(&bid_idx.u128().to_be_bytes());

    Ok(())
}

pub fn read_bid<S: ReadonlyStorage>(storage: &S, bid_idx: Uint128) -> StdResult<Bid> {
    let bid_bucket: ReadonlyBucket<S, Bid> = ReadonlyBucket::new(PREFIX_BID, storage);
    bid_bucket
        .load(&bid_idx.u128().to_be_bytes())
        .map_err(|_| StdError::generic_err("No bids with the specified information exist"))
}

pub fn read_bids_by_user<S: ReadonlyStorage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
    bidder: &CanonicalAddr,
    start_after: Option<Uint128>,
    limit: Option<u8>,
) -> StdResult<Vec<Bid>> {
    let bid_user_index: ReadonlyBucket<S, bool> = ReadonlyBucket::multilevel(
        &[
            PREFIX_BID_BY_USER,
            collateral_token.as_slice(),
            bidder.as_slice(),
        ],
        storage,
    );

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start_idx(start_after);

    bid_user_index
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, _) = elem?;
            read_bid(storage, Uint128(bytes_to_u128(&k)?))
        })
        .collect()
}

fn bytes_to_u128(data: &[u8]) -> StdResult<u128> {
    match data[0..16].try_into() {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 16 byte expected.",
        )),
    }
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start_idx(start_after: Option<Uint128>) -> Option<Vec<u8>> {
    start_after.map(|idx| {
        let mut v = idx.u128().to_be_bytes().to_vec();
        v.push(1);
        v
    })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<u8>) -> Option<Vec<u8>> {
    start_after.map(|id| {
        let mut v = id.to_be_bytes().to_vec();
        v.push(1);
        v
    })
}
