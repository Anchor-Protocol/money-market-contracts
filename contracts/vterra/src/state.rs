use std::collections::VecDeque;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Addr, CanonicalAddr, StdResult, Storage, Timestamp};
use cosmwasm_storage::{bucket, bucket_read, ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const KEY_CONFIG: &[u8] = b"config";
pub const KEY_STATE: &[u8] = b"state";

const PREFIX_USER_UNLOCK_INFOS: &[u8] = b"receipts";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub contract_addr: CanonicalAddr,
    pub owner_addr: CanonicalAddr,
    pub market_addr: CanonicalAddr,
    pub overseer_addr: CanonicalAddr,
    /// CW20 contract
    pub aterra_contract: CanonicalAddr,
    /// CW20 contract
    pub vterra_contract: CanonicalAddr,

    /// Maximum premium rate can increase per epoch
    pub max_pos_change: Decimal256,
    /// Maximum premium rate can decrease per epoch
    pub max_neg_change: Decimal256,
    /// Maximum premium rate
    pub max_rate: Decimal256,
    /// Minimum premium rate
    pub min_rate: Decimal256,
    /// Coefficient to multiply difference between target and current ve/a deposit share
    pub diff_multiplier: Decimal256,
    /// Number of blocks between updating premium rate
    pub premium_rate_epoch: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// Cached vterra supply.
    /// This is kept locally to not require expensive queries to CW20 contract
    pub vterra_supply: Uint256,
    /// Exchange rate between vterra and aterra calculated during last ExecuteEpochOperations
    pub prev_epoch_vterra_exchange_rate: Decimal256,
    /// Target share of deposits in vterra. o
    /// Premium rate adjusts to bring current share towards target share
    pub target_share: Decimal256,
    /// Current premium rate of vterra over aterra measured in blocks
    /// ex. 2% yearly premium => 1.02 / num_blocks_per_year
    pub premium_rate: Decimal256, // in blocks
    /// Block height ExecuteEpochOperations was last executed on
    pub last_updated: u64,
}

/// [INVARIANT]: receipts are stored in ascending order of unlock_time
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserReceipts(pub VecDeque<Receipt>);

/// Receipt given after unbonding vterra
/// Can be redeemed for aterra after block time has passed unlock time
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Receipt {
    pub aterra_qty: Uint256,
    pub unlock_time: Timestamp,
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

pub fn store_user_receipts(
    storage: &mut dyn Storage,
    user: &Addr,
    staker_info: &UserReceipts,
) -> StdResult<()> {
    bucket(storage, PREFIX_USER_UNLOCK_INFOS).save(user.as_bytes(), staker_info)
}

pub fn read_user_receipts(storage: &dyn Storage, user: &Addr) -> UserReceipts {
    match bucket_read(storage, PREFIX_USER_UNLOCK_INFOS).load(user.as_bytes()) {
        Ok(v) => v,
        _ => UserReceipts(VecDeque::new()),
    }
}
