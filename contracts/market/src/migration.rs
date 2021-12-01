use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{store_state, State, KEY_STATE};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::ReadonlySingleton;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct LegacyState {
    pub total_liabilities: Decimal256,
    pub total_reserves: Decimal256,
    pub last_interest_updated_time: u64,
    pub last_reward_updated_time: u64,
    pub global_interest_index: Decimal256,
    pub global_reward_index: Decimal256,
    pub anc_emission_rate: Decimal256,
    pub prev_aterra_supply: Uint256,
    pub prev_exchange_rate: Decimal256,
}

fn read_legacy_state(storage: &dyn Storage) -> StdResult<LegacyState> {
    ReadonlySingleton::new(storage, KEY_STATE).load()
}

pub fn migrate_state(storage: &mut dyn Storage, distributed_rewards: Uint256) -> StdResult<()> {
    let legacy_state: LegacyState = read_legacy_state(storage)?;

    store_state(
        storage,
        &State {
            total_liabilities: legacy_state.total_liabilities,
            total_reserves: legacy_state.total_reserves,
            last_interest_updated_time: legacy_state.last_interest_updated_time,
            last_reward_updated_time: legacy_state.last_reward_updated_time,
            global_interest_index: legacy_state.global_interest_index,
            global_reward_index: legacy_state.global_reward_index,
            anc_emission_rate: legacy_state.anc_emission_rate,
            prev_aterra_supply: legacy_state.prev_aterra_supply,
            prev_exchange_rate: legacy_state.prev_exchange_rate,
            distributed_rewards,
        },
    )
}
