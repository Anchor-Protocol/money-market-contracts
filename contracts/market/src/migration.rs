use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::ReadonlySingleton;

use crate::state::{store_config, store_state, Config, State, KEY_CONFIG, KEY_STATE};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct LegacyConfig {
    pub contract_addr: CanonicalAddr,
    pub owner_addr: CanonicalAddr,
    pub aterra_contract: CanonicalAddr,
    pub interest_model: CanonicalAddr,
    pub distribution_model: CanonicalAddr,
    pub overseer_contract: CanonicalAddr,
    pub collector_contract: CanonicalAddr,
    pub distributor_contract: CanonicalAddr,
    pub stable_denom: String,
    pub reserve_factor: Decimal256,
    pub max_borrow_factor: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct LegacyState {
    pub total_liabilities: Decimal256,
    pub total_reserves: Decimal256,
    pub last_interest_updated: u64,
    pub last_reward_updated: u64,
    pub global_interest_index: Decimal256,
    pub global_reward_index: Decimal256,
    pub anc_emission_rate: Decimal256,
}

fn read_legacy_config(storage: &dyn Storage) -> StdResult<LegacyConfig> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

fn read_legacy_state(storage: &dyn Storage) -> StdResult<LegacyState> {
    ReadonlySingleton::new(storage, KEY_STATE).load()
}

fn compute_exchange_rate_with_legacy_state(
    state: &LegacyState,
    aterra_supply: Uint256,
    contract_balance: Uint256,
) -> Decimal256 {
    if aterra_supply.is_zero() {
        return Decimal256::one();
    }

    // (aterra / stable_denom)
    // exchange_rate = (balance + total_liabilities - total_reserves) / aterra_supply
    (Decimal256::from_uint256(contract_balance) + state.total_liabilities - state.total_reserves)
        / Decimal256::from_uint256(aterra_supply)
}

pub fn migrate_config(
    storage: &mut dyn Storage,
    collector_contract: CanonicalAddr,
) -> StdResult<()> {
    let legacy_config: LegacyConfig = read_legacy_config(storage)?;
    store_config(
        storage,
        &Config {
            contract_addr: legacy_config.contract_addr,
            owner_addr: legacy_config.owner_addr,
            aterra_contract: legacy_config.aterra_contract,
            interest_model: legacy_config.interest_model,
            distribution_model: legacy_config.distribution_model,
            overseer_contract: legacy_config.overseer_contract,
            distributor_contract: legacy_config.distributor_contract,
            stable_denom: legacy_config.stable_denom,
            max_borrow_factor: legacy_config.max_borrow_factor,
            collector_contract,
        },
    )
}

pub fn migrate_state(
    storage: &mut dyn Storage,
    aterra_supply: Uint256,
    balance: Uint256,
) -> StdResult<()> {
    let legacy_state: LegacyState = read_legacy_state(storage)?;
    let exchange_rate =
        compute_exchange_rate_with_legacy_state(&legacy_state, aterra_supply, balance);

    store_state(
        storage,
        &State {
            total_liabilities: legacy_state.total_liabilities,
            total_reserves: legacy_state.total_reserves,
            last_interest_updated: legacy_state.last_interest_updated,
            last_reward_updated: legacy_state.last_reward_updated,
            global_interest_index: legacy_state.global_interest_index,
            global_reward_index: legacy_state.global_reward_index,
            anc_emission_rate: legacy_state.anc_emission_rate,
            prev_aterra_supply: aterra_supply,
            prev_exchange_rate: exchange_rate,
        },
    )
}
