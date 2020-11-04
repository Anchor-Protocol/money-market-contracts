use crate::msg::{CollateralsResponse, WhitelistResponseElem};
use cosmwasm_std::{
    Api, CanonicalAddr, Decimal, Extern, HumanAddr, Order, Querier, StdError, StdResult, Storage,
    Uint128,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const KEY_CONFIG: &[u8] = b"config";
const KEY_EPOCH_STATE: &[u8] = b"epoch_state";

const PREFIX_WHITELIST: &[u8] = b"whitelist";
const PREFIX_COLLATERALS: &[u8] = b"collateral";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_addr: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub base_denom: String,
    pub distribution_threshold: Decimal,
    pub target_deposit_rate: Decimal,
    pub buffer_distribution_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EpochState {
    pub deposit_rate: Decimal,
    pub prev_a_token_supply: Uint128,
    pub prev_exchange_rate: Decimal,
    pub last_executed_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistElem {
    pub ltv: Decimal,
    pub custody_contract: CanonicalAddr,
}

pub type Tokens = Vec<(CanonicalAddr, Uint128)>; // <(Collateral Token, Amount)>
pub trait TokensMath {
    fn sub(self: &mut Self, collaterals: Tokens) -> StdResult<()>;
    fn add(self: &mut Self, collaterals: Tokens);
}

impl TokensMath for Tokens {
    fn sub(self: &mut Self, tokens: Tokens) -> StdResult<()> {
        self.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut tokens = tokens.clone();
        tokens.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut i = 0;
        let mut j = 0;
        while i < self.len() || j < tokens.len() {
            if self[i].0 == tokens[j].0 {
                i += 1;
                j += 1;

                self[i].1 = (self[i].1 - tokens[j].1)?;
            } else if self[i].0.as_slice().cmp(&tokens[j].0.as_slice())
                == std::cmp::Ordering::Greater
            {
                j += 1;
            } else {
                i += 1;
            }
        }

        if j != tokens.len() {
            return Err(StdError::generic_err("Subtraction underflow"));
        }

        Ok(())
    }

    fn add(self: &mut Self, tokens: Tokens) {
        self.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut tokens = tokens.clone();
        tokens.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut i = 0;
        let mut j = 0;
        while i < self.len() || j < tokens.len() {
            if self[i].0 == tokens[j].0 {
                i += 1;
                j += 1;

                self[i].1 += tokens[j].1;
            } else if self[i].0.as_slice().cmp(&tokens[j].0.as_slice())
                == std::cmp::Ordering::Greater
            {
                j += 1;
            } else {
                i += 1;
            }
        }

        while j < tokens.len() {
            self.push(tokens[j].clone());
        }
    }
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
            let collaterals: Vec<(HumanAddr, Uint128)> = v
                .iter()
                .map(|c| Ok((deps.api.human_address(&c.0)?, c.1)))
                .collect::<StdResult<Vec<(HumanAddr, Uint128)>>>()?;

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
