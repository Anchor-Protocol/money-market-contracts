use cosmwasm_bignumber::{Decimal256, Uint256};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Storage;
use cosmwasm_std::{CanonicalAddr, DepsMut, StdResult};
use cosmwasm_storage::ReadonlySingleton;

use moneymarket::market::MigrateMsg;

use crate::querier::query_vterra_state;
use crate::state::{store_config, store_state, Config, State, KEY_CONFIG, KEY_STATE};

pub(crate) fn migrate(deps: DepsMut, block_height: u64, msg: MigrateMsg) -> StdResult<()> {
    let old_config: OldConfig = read_old_config(deps.storage)?;
    store_config(
        deps.storage,
        &Config {
            contract_addr: old_config.contract_addr,
            owner_addr: old_config.owner_addr,
            aterra_contract: old_config.aterra_contract,
            interest_model: old_config.interest_model,
            distribution_model: old_config.distribution_model,
            overseer_contract: old_config.overseer_contract,
            collector_contract: old_config.collector_contract,
            distributor_contract: old_config.distributor_contract,
            stable_denom: old_config.stable_denom,
            max_borrow_factor: old_config.max_borrow_factor,
            // new
            vterra_cw20_contract: deps.api.addr_canonicalize(&msg.vterra_cw20_addr)?,
            vterra_anchor_contract: deps.api.addr_canonicalize(&msg.vterra_anchor_addr)?,
        },
    )?;

    let vterra_state = query_vterra_state(
        deps.as_ref(),
        deps.api.addr_validate(&msg.vterra_anchor_addr)?,
    )?;
    let old_state = read_old_state(deps.storage)?;
    store_state(
        deps.storage,
        &State {
            total_liabilities: old_state.total_liabilities,
            total_reserves: old_state.total_reserves,
            last_interest_updated: old_state.last_interest_updated,
            last_reward_updated: old_state.last_reward_updated,
            global_interest_index: old_state.global_interest_index,
            global_reward_index: old_state.global_reward_index,
            anc_emission_rate: old_state.anc_emission_rate,
            prev_aterra_supply: old_state.prev_aterra_supply,
            prev_aterra_exchange_rate: old_state.prev_aterra_exchange_rate,
            // new
            prev_ve_premium_rate: vterra_state.premium_rate,
            prev_vterra_exchange_rate: vterra_state.prev_epoch_vterra_exchange_rate,
            prev_vterra_supply: vterra_state.vterra_supply,
            vterra_exchange_rate_last_updated: block_height,
        },
    )?;

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct OldConfig {
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
struct OldState {
    pub total_liabilities: Decimal256,
    pub total_reserves: Decimal256,
    pub last_interest_updated: u64,
    pub last_reward_updated: u64,
    pub global_interest_index: Decimal256,
    pub global_reward_index: Decimal256,
    pub anc_emission_rate: Decimal256,
    pub prev_aterra_supply: Uint256,
    pub prev_aterra_exchange_rate: Decimal256,
}

fn read_old_config(storage: &dyn Storage) -> StdResult<OldConfig> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

fn read_old_state(storage: &dyn Storage) -> StdResult<OldState> {
    ReadonlySingleton::new(storage, KEY_STATE).load()
}
