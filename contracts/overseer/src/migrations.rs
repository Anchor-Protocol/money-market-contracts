use cosmwasm_bignumber::Decimal256;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Storage;
use cosmwasm_std::{CanonicalAddr, DepsMut, StdResult};
use cosmwasm_storage::ReadonlySingleton;

use moneymarket::overseer::MigrateMsg;

use crate::state::{store_config, Config, KEY_CONFIG};

pub(crate) fn migrate(deps: DepsMut, msg: MigrateMsg) -> StdResult<()> {
    let old_config: OldConfig = read_old_config(deps.storage)?;
    store_config(
        deps.storage,
        &Config {
            owner_addr: old_config.owner_addr,
            oracle_contract: old_config.oracle_contract,
            market_contract: old_config.market_contract,
            liquidation_contract: old_config.liquidation_contract,
            collector_contract: old_config.collector_contract,
            stable_denom: old_config.stable_denom,
            epoch_period: old_config.epoch_period,
            threshold_deposit_rate: old_config.threshold_deposit_rate,
            target_deposit_rate: old_config.target_deposit_rate,
            buffer_distribution_factor: old_config.buffer_distribution_factor,
            anc_purchase_factor: old_config.anc_purchase_factor,
            price_timeframe: old_config.price_timeframe,
            // new
            vterra_contract: deps.api.addr_canonicalize(&msg.vterra_contract_addr)?,
        },
    )?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct OldConfig {
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

fn read_old_config(storage: &dyn Storage) -> StdResult<OldConfig> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}
