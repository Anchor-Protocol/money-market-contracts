use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    Api, CanonicalAddr, Extern, HumanAddr, Order, Querier, StdError, StdResult, Storage,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};
use moneymarket::Tokens;

use crate::msg::{CollateralsResponse, WhitelistResponseElem};

const KEY_CONFIG: &[u8] = b"config";
const KEY_EPOCH_STATE: &[u8] = b"epoch_state";

const PREFIX_WHITELIST: &[u8] = b"whitelist";
const PREFIX_COLLATERALS: &[u8] = b"collateral";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_addr: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub liquidation_contract: CanonicalAddr,
    pub stable_denom: String,
    pub epoch_period: u64,
    pub distribution_threshold: Decimal256,
    pub target_deposit_rate: Decimal256,
    pub buffer_distribution_rate: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EpochState {
    pub deposit_rate: Decimal256,
    pub prev_a_token_supply: Uint256,
    pub prev_exchange_rate: Decimal256,
    pub last_executed_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistElem {
    pub ltv: Decimal256,
    pub custody_contract: CanonicalAddr,
}

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_epoch_state<S: Storage>(storage: &mut S, data: &EpochState) -> StdResult<()> {
    Singleton::new(storage, KEY_EPOCH_STATE).save(data)
}

pub fn read_epoch_state<S: Storage>(storage: &S) -> StdResult<EpochState> {
    ReadonlySingleton::new(storage, KEY_EPOCH_STATE).load()
}

pub fn store_whitelist_elem<S: Storage>(
    storage: &mut S,
    collateral_token: &CanonicalAddr,
    whitelist_elem: &WhitelistElem,
) -> StdResult<()> {
    let mut whitelist_bucket: Bucket<S, WhitelistElem> = Bucket::new(PREFIX_WHITELIST, storage);
    whitelist_bucket.save(collateral_token.as_slice(), &whitelist_elem)?;

    Ok(())
}

pub fn read_whitelist_elem<S: Storage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
) -> StdResult<WhitelistElem> {
    let whitelist_bucket: ReadonlyBucket<S, WhitelistElem> =
        ReadonlyBucket::new(PREFIX_WHITELIST, storage);
    match whitelist_bucket.load(&collateral_token.as_slice()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err("No whitelist data is stored")),
    }
}

pub fn read_whitelist<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<WhitelistResponseElem>> {
    let whitelist_bucket: ReadonlyBucket<S, WhitelistElem> =
        ReadonlyBucket::new(PREFIX_WHITELIST, &deps.storage);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let collateral_token: HumanAddr = deps.api.human_address(&CanonicalAddr::from(k))?;
            let custody_contract: HumanAddr = deps.api.human_address(&v.custody_contract)?;
            Ok(WhitelistResponseElem {
                collateral_token,
                custody_contract,
                ltv: v.ltv,
            })
        })
        .collect()
}

#[allow(clippy::ptr_arg)]
pub fn store_collaterals<S: Storage>(
    storage: &mut S,
    borrower: &CanonicalAddr,
    collaterals: &Tokens,
) -> StdResult<()> {
    let mut collaterals_bucket: Bucket<S, Tokens> = Bucket::new(PREFIX_COLLATERALS, storage);
    collaterals_bucket.save(&borrower.as_slice(), &collaterals)?;

    Ok(())
}

pub fn read_collaterals<S: Storage>(storage: &S, borrower: &CanonicalAddr) -> Tokens {
    let collaterals_bucket: ReadonlyBucket<S, Tokens> =
        ReadonlyBucket::new(PREFIX_COLLATERALS, storage);
    match collaterals_bucket.load(&borrower.as_slice()) {
        Ok(v) => v,
        _ => vec![],
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_all_collaterals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<CollateralsResponse>> {
    let whitelist_bucket: ReadonlyBucket<S, Tokens> =
        ReadonlyBucket::new(PREFIX_COLLATERALS, &deps.storage);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let borrower: HumanAddr = deps.api.human_address(&CanonicalAddr::from(k))?;
            let collaterals: Vec<(HumanAddr, Uint256)> = v
                .iter()
                .map(|c| Ok((deps.api.human_address(&c.0)?, c.1)))
                .collect::<StdResult<Vec<(HumanAddr, Uint256)>>>()?;

            Ok(CollateralsResponse {
                borrower,
                collaterals,
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
