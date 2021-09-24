use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, Order, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, ReadonlyBucket, ReadonlySingleton, Singleton};

use moneymarket::market::BorrowerInfoResponse;

pub const KEY_CONFIG: &[u8] = b"config";
pub const KEY_STATE: &[u8] = b"state";

const PREFIX_LIABILITY: &[u8] = b"liability";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub contract_addr: CanonicalAddr,
    pub owner_addr: CanonicalAddr,
    pub aterra_contract: CanonicalAddr,
    pub interest_model: CanonicalAddr,
    pub distribution_model: CanonicalAddr,
    pub overseer_contract: CanonicalAddr,
    pub collector_contract: CanonicalAddr,
    pub distributor_contract: CanonicalAddr,
    pub stable_denom: String,
    pub max_borrow_factor: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_liabilities: Decimal256,
    pub total_reserves: Decimal256,
    pub last_interest_updated: u64,
    pub last_reward_updated: u64,
    pub global_interest_index: Decimal256,
    pub global_reward_index: Decimal256,
    pub anc_emission_rate: Decimal256,
    pub prev_aterra_supply: Uint256,
    pub prev_exchange_rate: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerInfo {
    pub interest_index: Decimal256,
    pub reward_index: Decimal256,
    pub loan_amount: Uint256,
    pub pending_rewards: Decimal256,
}

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_state(storage: &mut dyn Storage, data: &State) -> StdResult<()> {
    Singleton::new(storage, KEY_STATE).save(data)
}

pub fn read_state(storage: &dyn Storage) -> StdResult<State> {
    ReadonlySingleton::new(storage, KEY_STATE).load()
}

pub fn store_borrower_info(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    liability: &BorrowerInfo,
) -> StdResult<()> {
    bucket(storage, PREFIX_LIABILITY).save(borrower.as_slice(), liability)
}

pub fn read_borrower_info(storage: &dyn Storage, borrower: &CanonicalAddr) -> BorrowerInfo {
    match bucket_read(storage, PREFIX_LIABILITY).load(borrower.as_slice()) {
        Ok(v) => v,
        _ => BorrowerInfo {
            interest_index: Decimal256::one(),
            reward_index: Decimal256::zero(),
            loan_amount: Uint256::zero(),
            pending_rewards: Decimal256::zero(),
        },
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_borrower_infos(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<BorrowerInfoResponse>> {
    let liability_bucket: ReadonlyBucket<BorrowerInfo> =
        bucket_read(deps.storage, PREFIX_LIABILITY);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    liability_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let borrower = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            Ok(BorrowerInfoResponse {
                borrower,
                interest_index: v.interest_index,
                reward_index: v.reward_index,
                loan_amount: v.loan_amount,
                pending_rewards: v.pending_rewards,
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
