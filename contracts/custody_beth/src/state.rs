use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use moneymarket::custody::{BAssetInfo, BorrowerResponse};

//BETHAccruedRewardsResponse the struct that shows the result of accrued_rewards query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BETHAccruedRewardsResponse {
    pub rewards: Uint128,
}

const PREFIX_BORROWER: &[u8] = b"borrower";
const KEY_CONFIG: &[u8] = b"config";

const REWARDS_INFO: Item<RewardsInfo> = Item::new("rewards_info");
const USER_REWARDS: Map<&[u8], UserRewards> = Map::new("user_rewards");

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, JsonSchema)]
pub struct RewardsInfo {
    pub global_index: Decimal256,
    pub cumulative_total: Uint256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, JsonSchema)]
pub struct UserRewards {
    pub user_index: Decimal256,
    pub rewards: Uint256,
}

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_borrower_info(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    borrower_info: &BorrowerInfo,
) -> StdResult<()> {
    let mut borrower_bucket: Bucket<BorrowerInfo> = Bucket::new(storage, PREFIX_BORROWER);
    borrower_bucket.save(borrower.as_slice(), borrower_info)?;

    Ok(())
}

pub fn remove_borrower_info(storage: &mut dyn Storage, borrower: &CanonicalAddr) {
    let mut borrower_bucket: Bucket<BorrowerInfo> = Bucket::new(storage, PREFIX_BORROWER);
    borrower_bucket.remove(borrower.as_slice());
}

pub fn read_borrower_info(storage: &dyn Storage, borrower: &CanonicalAddr) -> BorrowerInfo {
    let borrower_bucket: ReadonlyBucket<BorrowerInfo> =
        ReadonlyBucket::new(storage, PREFIX_BORROWER);
    match borrower_bucket.load(borrower.as_slice()) {
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
pub fn read_borrowers(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<BorrowerResponse>> {
    let position_bucket: ReadonlyBucket<BorrowerInfo> =
        ReadonlyBucket::new(deps.storage, PREFIX_BORROWER);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    position_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            let borrower: CanonicalAddr = CanonicalAddr::from(k);
            Ok(BorrowerResponse {
                borrower: deps.api.addr_humanize(&borrower)?.to_string(),
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

pub fn save_rewards_info(storage: &mut dyn Storage, data: &RewardsInfo) -> StdResult<()> {
    REWARDS_INFO.save(storage, data)
}

pub fn read_rewards_info(storage: &dyn Storage) -> StdResult<RewardsInfo> {
    REWARDS_INFO
        .may_load(storage)
        .map(Option::unwrap_or_default)
}

pub fn update_rewards_info(
    storage: &mut dyn Storage,
    reward_amount: &Uint256,
    collateral_amount: &Uint256,
) -> StdResult<()> {
    let mut current = REWARDS_INFO.may_load(storage)?.unwrap_or_default();
    current.global_index += Decimal256::from_ratio(*reward_amount, *collateral_amount);
    current.cumulative_total += *reward_amount;
    save_rewards_info(storage, &current)
}

pub fn save_user_rewards(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    new_rewards: &UserRewards,
) -> StdResult<()> {
    USER_REWARDS.save(storage, borrower.as_slice(), new_rewards)
}

pub fn read_user_rewards(
    storage: &dyn Storage,
    borrower: &CanonicalAddr,
) -> StdResult<UserRewards> {
    USER_REWARDS
        .may_load(storage, borrower.as_slice())
        .map(Option::unwrap_or_default)
}

pub fn update_user_rewards(storage: &mut dyn Storage, borrower: &CanonicalAddr) -> StdResult<()> {
    let global_index = read_rewards_info(storage)?.global_index;
    let mut user_rewards = read_user_rewards(storage, borrower)?;
    let borrower_info = read_borrower_info(storage, borrower);
    user_rewards.rewards += borrower_info.balance * (global_index - user_rewards.user_index);
    user_rewards.user_index = global_index;
    save_user_rewards(storage, borrower, &user_rewards)
}
