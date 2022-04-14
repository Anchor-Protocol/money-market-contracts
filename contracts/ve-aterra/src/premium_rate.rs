use cosmwasm_bignumber::{Decimal256, Uint256};

use crate::state::{Config, State};

pub fn update_ve_premium_rate(state: &mut State, config: Config, aterra_supply: Uint256) {
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
    Decimal256::from_ratio(
        state.ve_aterra_supply * state.prev_epoch_ve_aterra_exchange_rate,
        aterra_supply.max(Uint256::one()),
    )
}
