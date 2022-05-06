use crate::bonding::compute_ve_exchange_rate;
use crate::error::ContractError;
use crate::querier::query_supply;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::state::{read_config, read_state, store_state, Config, State};

pub fn execute_epoch_operations(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut state = read_state(deps.storage)?;
    let config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(_info.sender.as_str())? != config.overseer_addr {
        return Err(ContractError::Unauthorized {});
    }

    if state.last_updated + config.premium_rate_epoch > env.block.height {
        return Err(ContractError::EpochNotPassed(state.last_updated));
    }

    // store new exchange rate BEFORE updating premium rate
    state.prev_epoch_vterra_exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    // ensure cached vterra_supply is equal to ground truth
    state.vterra_supply = query_supply(
        deps.as_ref(),
        deps.api.addr_humanize(&config.vterra_contract)?,
    )?;
    // aterra_supply used to calculate current ve vs. aterra deposit share
    let aterra_supply = query_supply(
        deps.as_ref(),
        deps.api.addr_humanize(&config.aterra_contract)?,
    )?;
    // let base_rate = config.overseer_addr
    update_vterra_premium_rate(&mut state, config, aterra_supply);

    state.last_updated = env.block.height;

    store_state(deps.storage, &state)?;
    Ok(Response::new())
}

pub fn update_vterra_premium_rate(state: &mut State, config: Config, aterra_supply: Uint256) {
    let current_share = current_ve_share(state, aterra_supply);

    // update target_share every overseer epoch
    let raw_rate = if state.target_share > current_share {
        let delta = (config.diff_multiplier * (state.target_share - current_share))
            .min(config.max_pos_change);
        state.premium_rate + delta
    } else {
        let delta = (config.diff_multiplier * (current_share - state.target_share))
            .min(config.max_neg_change);
        state.premium_rate - delta
    };
    state.premium_rate = raw_rate.max(config.min_rate).min(config.max_rate);
}

pub fn current_ve_share(state: &State, aterra_supply: Uint256) -> Decimal256 {
    let converted_ve = state.vterra_supply * state.prev_epoch_vterra_exchange_rate;
    Decimal256::from_ratio(converted_ve, converted_ve + aterra_supply)
}
