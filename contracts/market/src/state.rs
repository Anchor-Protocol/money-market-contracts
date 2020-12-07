use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{
    Api, CanonicalAddr, Extern, HumanAddr, Order, Querier, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};

use crate::msg::LiabilityResponse;

const KEY_CONFIG: &[u8] = b"config";
const KEY_STATE: &[u8] = b"state";

const PREFIX_LIABILITY: &[u8] = b"liability";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub contract_addr: CanonicalAddr,
    pub owner_addr: CanonicalAddr,
    pub anchor_token: CanonicalAddr,
    pub interest_model: CanonicalAddr,
    pub overseer_contract: CanonicalAddr,
    pub stable_denom: String,
    pub reserve_factor: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_liabilities: Decimal256,
    pub total_reserves: Decimal256,
    pub last_interest_updated: u64,
    pub global_interest_index: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Liability {
    pub interest_index: Decimal256,
    pub loan_amount: Uint128,
}

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_state<S: Storage>(storage: &mut S, data: &State) -> StdResult<()> {
    Singleton::new(storage, KEY_STATE).save(data)
}

pub fn read_state<S: Storage>(storage: &S) -> StdResult<State> {
    ReadonlySingleton::new(storage, KEY_STATE).load()
}

pub fn store_liability<S: Storage>(
    storage: &mut S,
    borrower: &CanonicalAddr,
    liability: &Liability,
) -> StdResult<()> {
    let mut liability_bucket: Bucket<S, Liability> = Bucket::new(PREFIX_LIABILITY, storage);
    liability_bucket.save(borrower.as_slice(), &liability)?;

    Ok(())
}

pub fn read_liability<S: Storage>(storage: &S, borrower: &CanonicalAddr) -> Liability {
    let liability_bucket: ReadonlyBucket<S, Liability> =
        ReadonlyBucket::new(PREFIX_LIABILITY, storage);
    match liability_bucket.load(&borrower.as_slice()) {
        Ok(v) => v,
        _ => Liability {
            interest_index: Decimal256::one(),
            loan_amount: Uint128::zero(),
        },
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_liabilities<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<LiabilityResponse>> {
    let liability_bucket: ReadonlyBucket<S, Liability> =
        ReadonlyBucket::new(PREFIX_LIABILITY, &deps.storage);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    liability_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let borrower: HumanAddr = deps.api.human_address(&CanonicalAddr::from(k))?;
            Ok(LiabilityResponse {
                borrower,
                interest_index: v.interest_index,
                loan_amount: v.loan_amount,
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
