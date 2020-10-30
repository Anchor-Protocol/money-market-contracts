use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Api, CanonicalAddr, Decimal, Extern, Order, Querier, StdResult, Storage, Uint128,
};

use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};

use crate::msg::BorrowerResponse;

const KEY_CONFIG: &[u8] = b"config";
const KEY_GLOBAL_INDEX: &[u8] = b"global_index";
const PREFIX_BORROWER: &[u8] = b"borrower";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub asset_token: CanonicalAddr,
    pub overseer_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub reward_contract: CanonicalAddr,
    pub reward_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerInfo {
    pub balance: Uint128,
    pub spendable: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
}

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn increase_global_index<S: Storage>(storage: &mut S, amount: Decimal) -> StdResult<()> {
    let global_index = read_global_index(storage);
    Singleton::new(storage, KEY_GLOBAL_INDEX).save(&(global_index + amount))
}

pub fn read_global_index<S: Storage>(storage: &S) -> Decimal {
    match ReadonlySingleton::new(storage, KEY_GLOBAL_INDEX).load() {
        Ok(v) => v,
        _ => Decimal::zero(),
    }
}

pub fn store_borrower_info<S: Storage>(
    storage: &mut S,
    borrower: &CanonicalAddr,
    borrower_info: &BorrowerInfo,
) -> StdResult<()> {
    let mut borrower_bucket: Bucket<S, BorrowerInfo> = Bucket::new(PREFIX_BORROWER, storage);
    borrower_bucket.save(borrower.as_slice(), &borrower_info)?;

    Ok(())
}

pub fn remove_borrower_info<S: Storage>(storage: &mut S, borrower: &CanonicalAddr) {
    let mut borrower_bucket: Bucket<S, BorrowerInfo> = Bucket::new(PREFIX_BORROWER, storage);
    borrower_bucket.remove(borrower.as_slice());
}

pub fn read_borrower_info<S: Storage>(storage: &S, borrower: &CanonicalAddr) -> BorrowerInfo {
    let borrower_bucket: ReadonlyBucket<S, BorrowerInfo> =
        ReadonlyBucket::new(PREFIX_BORROWER, storage);
    match borrower_bucket.load(&borrower.as_slice()) {
        Ok(v) => v,
        _ => BorrowerInfo {
            balance: Uint128::zero(),
            spendable: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        },
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_borrowers<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<BorrowerResponse>> {
    let position_bucket: ReadonlyBucket<S, BorrowerInfo> =
        ReadonlyBucket::new(PREFIX_BORROWER, &deps.storage);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    position_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            let borrower: CanonicalAddr = CanonicalAddr::from(k);
            Ok(BorrowerResponse {
                borrower: deps.api.human_address(&borrower)?,
                balance: v.balance,
                spendable: v.spendable,
                reward_index: v.reward_index,
                pending_reward: v.pending_reward,
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
