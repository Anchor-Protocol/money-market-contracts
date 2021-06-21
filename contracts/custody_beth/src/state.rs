use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    Api, CanonicalAddr, Decimal, Extern, Order, Querier, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};
use moneymarket::custody::{BAssetInfo, BorrowerResponse};

//BETHState the struct that beth-reward contract stores the state of the contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BETHState {
    pub global_index: Decimal,
    pub total_balance: Uint128,
    pub prev_reward_balance: Uint128,
}

const KEY_CONFIG: &[u8] = b"config";
const PREFIX_BORROWER: &[u8] = b"borrower";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub collateral_token: CanonicalAddr,
    pub overseer_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub reward_contract: CanonicalAddr,
    pub liquidation_contract: CanonicalAddr,
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerInfo {
    pub balance: Uint256,
    pub spendable: Uint256,
}

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
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
            balance: Uint256::zero(),
            spendable: Uint256::zero(),
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
