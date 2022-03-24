use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, Order, StdError, StdResult, Storage};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};

use moneymarket::overseer::{CollateralsResponse, WhitelistResponseElem};
use moneymarket::tokens::Tokens;

const KEY_CONFIG: &[u8] = b"config";
const KEY_DYNRATE_CONFIG: &[u8] = b"dynrate_config";
const KEY_EPOCH_STATE: &[u8] = b"epoch_state";
const KEY_DYNRATE_STATE: &[u8] = b"dynrate_state";

const PREFIX_WHITELIST: &[u8] = b"whitelist";
const PREFIX_COLLATERALS: &[u8] = b"collateral";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_addr: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub liquidation_contract: CanonicalAddr,
    pub collector_contract: CanonicalAddr,
    pub stable_denom: String,
    pub epoch_period: u64,
    pub threshold_deposit_rate: Decimal256,
    pub target_deposit_rate: Decimal256,
    pub buffer_distribution_factor: Decimal256,
    pub anc_purchase_factor: Decimal256,
    pub price_timeframe: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynrateConfig {
    pub dyn_rate_epoch: u64,
    pub dyn_rate_maxchange: Decimal256,
    pub dyn_rate_yr_increase_expectation: Decimal256,
    // clamps the deposit rate (in blocks)
    pub dyn_rate_min: Decimal256,
    pub dyn_rate_max: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EpochState {
    pub deposit_rate: Decimal256,
    pub prev_aterra_supply: Uint256,
    pub prev_exchange_rate: Decimal256,
    pub prev_interest_buffer: Uint256,
    pub last_executed_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynrateState {
    pub last_executed_height: u64,
    pub prev_yield_reserve: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistElem {
    pub name: String,
    pub symbol: String,
    pub max_ltv: Decimal256,
    pub custody_contract: CanonicalAddr,
}

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_dynrate_config(storage: &mut dyn Storage, data: &DynrateConfig) -> StdResult<()> {
    Singleton::new(storage, KEY_DYNRATE_CONFIG).save(data)
}

pub fn read_dynrate_config(storage: &dyn Storage) -> StdResult<DynrateConfig> {
    ReadonlySingleton::new(storage, KEY_DYNRATE_CONFIG).load()
}

pub fn store_epoch_state(storage: &mut dyn Storage, data: &EpochState) -> StdResult<()> {
    Singleton::new(storage, KEY_EPOCH_STATE).save(data)
}

pub fn read_epoch_state(storage: &dyn Storage) -> StdResult<EpochState> {
    ReadonlySingleton::new(storage, KEY_EPOCH_STATE).load()
}

pub fn store_dynrate_state(storage: &mut dyn Storage, data: &DynrateState) -> StdResult<()> {
    Singleton::new(storage, KEY_DYNRATE_STATE).save(data)
}

pub fn read_dynrate_state(storage: &dyn Storage) -> StdResult<DynrateState> {
    ReadonlySingleton::new(storage, KEY_DYNRATE_STATE).load()
}

pub fn store_whitelist_elem(
    storage: &mut dyn Storage,
    collateral_token: &CanonicalAddr,
    whitelist_elem: &WhitelistElem,
) -> StdResult<()> {
    let mut whitelist_bucket: Bucket<WhitelistElem> = Bucket::new(storage, PREFIX_WHITELIST);
    whitelist_bucket.save(collateral_token.as_slice(), whitelist_elem)?;

    Ok(())
}

pub fn read_whitelist_elem(
    storage: &dyn Storage,
    collateral_token: &CanonicalAddr,
) -> StdResult<WhitelistElem> {
    let whitelist_bucket: ReadonlyBucket<WhitelistElem> =
        ReadonlyBucket::new(storage, PREFIX_WHITELIST);
    match whitelist_bucket.load(collateral_token.as_slice()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err(
            "Token is not registered as collateral",
        )),
    }
}

pub fn read_whitelist(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<WhitelistResponseElem>> {
    let whitelist_bucket: ReadonlyBucket<WhitelistElem> =
        ReadonlyBucket::new(deps.storage, PREFIX_WHITELIST);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let collateral_token = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            let custody_contract = deps.api.addr_humanize(&v.custody_contract)?.to_string();
            Ok(WhitelistResponseElem {
                name: v.name,
                symbol: v.symbol,
                collateral_token,
                custody_contract,
                max_ltv: v.max_ltv,
            })
        })
        .collect()
}

#[allow(clippy::ptr_arg)]
pub fn store_collaterals(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    collaterals: &Tokens,
) -> StdResult<()> {
    let mut collaterals_bucket: Bucket<Tokens> = Bucket::new(storage, PREFIX_COLLATERALS);
    if collaterals.is_empty() {
        collaterals_bucket.remove(borrower.as_slice());
    } else {
        collaterals_bucket.save(borrower.as_slice(), collaterals)?;
    }

    Ok(())
}

pub fn read_collaterals(storage: &dyn Storage, borrower: &CanonicalAddr) -> Tokens {
    let collaterals_bucket: ReadonlyBucket<Tokens> =
        ReadonlyBucket::new(storage, PREFIX_COLLATERALS);
    match collaterals_bucket.load(borrower.as_slice()) {
        Ok(v) => v,
        _ => vec![],
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_all_collaterals(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<CollateralsResponse>> {
    let whitelist_bucket: ReadonlyBucket<Tokens> =
        ReadonlyBucket::new(deps.storage, PREFIX_COLLATERALS);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let borrower = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            let collaterals: Vec<(String, Uint256)> = v
                .iter()
                .map(|c| Ok((deps.api.addr_humanize(&c.0)?.to_string(), c.1)))
                .collect::<StdResult<Vec<(String, Uint256)>>>()?;

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
