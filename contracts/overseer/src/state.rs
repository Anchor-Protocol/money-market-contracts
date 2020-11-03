use crate::msg::{LoanResponse, WhitelistResponseItem};
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
const PREFIX_LOAN: &[u8] = b"loan";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_addr: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub reward_denom: String,
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
pub struct WhitelistItem {
    pub ltv: Decimal,
    pub custody_contract: CanonicalAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Loan {
    pub borrow_amount: Uint128,
    pub collaterals: Vec<(CanonicalAddr, Uint128)>, // <(Collateral Token, Amount)>
}

impl Loan {
    pub fn add_collateral(self: &mut Self, collaterals: Vec<(CanonicalAddr, Uint128)>) {
        self.collaterals
            .sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut collaterals = collaterals.clone();
        collaterals.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut i = 0;
        let mut j = 0;
        while i < self.collaterals.len() || j < collaterals.len() {
            if self.collaterals[i].0 == collaterals[j].0 {
                i += 1;
                j += 1;

                self.collaterals[i].1 += collaterals[j].1;
            } else if self.collaterals[i]
                .0
                .as_slice()
                .cmp(&collaterals[j].0.as_slice())
                == std::cmp::Ordering::Greater
            {
                j += 1;
            } else {
                i += 1;
            }
        }

        while j < collaterals.len() {
            self.collaterals.push(collaterals[j].clone());
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

pub fn store_whitelist_item<S: Storage>(
    storage: &mut S,
    collateral_token: &CanonicalAddr,
    whitelist_item: &WhitelistItem,
) -> StdResult<()> {
    let mut whitelist_bucket: Bucket<S, WhitelistItem> = Bucket::new(PREFIX_WHITELIST, storage);
    whitelist_bucket.save(collateral_token.as_slice(), &whitelist_item)?;

    Ok(())
}

pub fn read_whitelist_item<S: Storage>(
    storage: &S,
    collateral_token: &CanonicalAddr,
) -> StdResult<WhitelistItem> {
    let whitelist_bucket: ReadonlyBucket<S, WhitelistItem> =
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
) -> StdResult<Vec<WhitelistResponseItem>> {
    let whitelist_bucket: ReadonlyBucket<S, WhitelistItem> =
        ReadonlyBucket::new(PREFIX_WHITELIST, &deps.storage);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            let collateral_token: HumanAddr = deps.api.human_address(&CanonicalAddr::from(k))?;
            let custody_contract: HumanAddr = deps.api.human_address(&v.custody_contract)?;
            Ok(WhitelistResponseItem {
                collateral_token,
                custody_contract,
                ltv: v.ltv,
            })
        })
        .collect()
}

pub fn store_loan<S: Storage>(
    storage: &mut S,
    borrower: &CanonicalAddr,
    loan: &Loan,
) -> StdResult<()> {
    let mut loan_bucket: Bucket<S, Loan> = Bucket::new(PREFIX_LOAN, storage);
    loan_bucket.save(&borrower.as_slice(), &loan)?;

    Ok(())
}

pub fn read_loan<S: Storage>(storage: &S, borrower: &CanonicalAddr) -> Loan {
    let loan_bucket: ReadonlyBucket<S, Loan> = ReadonlyBucket::new(PREFIX_LOAN, storage);
    match loan_bucket.load(&borrower.as_slice()) {
        Ok(v) => v,
        _ => Loan {
            borrow_amount: Uint128::zero(),
            collaterals: vec![],
        },
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_loans<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<LoanResponse>> {
    let whitelist_bucket: ReadonlyBucket<S, Loan> = ReadonlyBucket::new(PREFIX_LOAN, &deps.storage);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            let borrower: HumanAddr = deps.api.human_address(&CanonicalAddr::from(k))?;
            let collaterals: Vec<(HumanAddr, Uint128)> = v
                .collaterals
                .iter()
                .map(|c| Ok((deps.api.human_address(&c.0)?, c.1)))
                .collect::<StdResult<Vec<(HumanAddr, Uint128)>>>()?;

            Ok(LoanResponse {
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
