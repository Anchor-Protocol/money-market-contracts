use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Addr, CanonicalAddr, StdResult, Storage, Timestamp};
use cosmwasm_storage::{bucket, bucket_read, ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const KEY_CONFIG: &[u8] = b"config";
pub const KEY_STATE: &[u8] = b"state";

const PREFIX_VE_ATERRA_STAKER: &[u8] = b"ve_aterra_staker";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub contract_addr: CanonicalAddr,
    pub owner_addr: CanonicalAddr,
    pub market_addr: CanonicalAddr,
    pub overseer_addr: CanonicalAddr,
    pub aterra_contract: CanonicalAddr,
    pub ve_aterra_contract: CanonicalAddr,

    pub max_pos_change: Decimal256,
    pub max_neg_change: Decimal256,
    pub max_rate: Decimal256,
    pub min_rate: Decimal256,
    pub diff_multiplier: Decimal256,

    pub premium_rate_epoch: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub ve_aterra_supply: Uint256,
    pub prev_epoch_ve_aterra_exchange_rate: Decimal256,
    pub target_share: Decimal256,
    pub premium_rate: Decimal256, // in blocks
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VeStakerUnlockInfos {
    pub infos: Vec<VeStakerUnlockInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VeStakerUnlockInfo {
    pub aterra_qty: Uint256,
    pub unlock_time: Timestamp,
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

pub fn store_ve_stacker_infos(
    storage: &mut dyn Storage,
    owner: &Addr,
    staker_info: &VeStakerUnlockInfos,
) -> StdResult<()> {
    bucket(storage, PREFIX_VE_ATERRA_STAKER).save(owner.as_bytes(), staker_info)
}

pub fn read_ve_aterra_staker_infos(storage: &dyn Storage, staker: &Addr) -> VeStakerUnlockInfos {
    match bucket_read(storage, PREFIX_VE_ATERRA_STAKER).load(staker.as_bytes()) {
        Ok(v) => v,
        _ => VeStakerUnlockInfos { infos: Vec::new() },
    }
}
