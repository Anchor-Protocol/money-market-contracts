use std::cmp::Ordering;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, Timestamp, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use moneymarket::market::ExecuteMsg;

use crate::error::ContractError;
use crate::state::{
    read_config, read_state, read_ve_aterra_staker_infos, store_state, store_ve_stacker_infos,
    Config, State, VeStakerUnlockInfo,
};

pub fn bond_aterra(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    bond_amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let ve_aterra_amount = bond_amount / exchange_rate;

    state.prev_ve_aterra_supply += ve_aterra_amount;
    store_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages([
            // burn bonded aterra
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: bond_amount.into(),
                })?,
            }),
            // mint ve aterra to bonder
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.ve_aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: sender.to_string(),
                    amount: ve_aterra_amount.into(),
                })?,
            }),
            // subtract bond amount from aterra supply stored in market contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.market_addr)?.into(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateAterraSupply {
                    diff: moneymarket::market::Diff::Neg(bond_amount),
                })?,
            }),
        ])
        .add_attributes([
            attr("action", "bond_aterra"),
            attr("depositor", sender),
            attr("bond_amount", bond_amount),
            attr("mint_amount", ve_aterra_amount),
        ]))
}

const UNBOND_DURATION_SECS: u64 = 60 * 60 * 24 * 30;

pub fn unbond_ve_aterra(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    unbond_amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    state.prev_ve_aterra_supply = state.prev_ve_aterra_supply - unbond_amount;
    store_state(deps.storage, &state)?;

    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let aterra_mint_amount = unbond_amount * exchange_rate;

    let mut staker_infos = read_ve_aterra_staker_infos(deps.storage, &sender);
    let unlock_time = env.block.time.plus_seconds(UNBOND_DURATION_SECS);
    staker_infos.infos.push(VeStakerUnlockInfo {
        aterra_qty: aterra_mint_amount,
        unlock_time,
    });
    store_ve_stacker_infos(deps.storage, &sender, &staker_infos)?;

    Ok(Response::new()
        .add_messages([
            // Burn ve aterra
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.ve_aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: unbond_amount.into(),
                })?,
            }),
            // Mint and custody aterra. This can be claimed 30 days from now
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: config.contract_addr.to_string(),
                    amount: aterra_mint_amount.into(),
                })?,
            }),
            // add aterra mint amount to aterra supply stored in market contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.market_addr)?.into(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateAterraSupply {
                    diff: moneymarket::market::Diff::Pos(aterra_mint_amount),
                })?,
            }),
        ])
        .add_attributes([
            attr("action", "unbond_ve_aterra"),
            attr("depositor", sender.to_string()),
            attr("unbond_amount", unbond_amount),
            attr("mint_amount", aterra_mint_amount),
            attr("unlock_time", unlock_time.to_string()),
        ]))
}

pub fn claim_unlocked_aterra(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unlock_timestamp: Timestamp,
    amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let mut staker_infos = read_ve_aterra_staker_infos(deps.storage, &info.sender);

    let staker_info_idx = staker_infos
        .infos
        .iter_mut()
        .position(|x| x.unlock_time == unlock_timestamp)
        .ok_or(ContractError::NoUnlockMatchingBlockHeight)?;

    if unlock_timestamp > env.block.time {
        return Err(ContractError::VeStakeNotUnlocked(
            env.block.time,
            unlock_timestamp,
        ));
    }

    let total_amount = staker_infos.infos[staker_info_idx].aterra_qty;
    match amount.cmp(&total_amount) {
        Ordering::Less => {
            let stored_qty = &mut staker_infos.infos[staker_info_idx].aterra_qty;
            *stored_qty = *stored_qty - amount;
        }
        Ordering::Equal => {
            staker_infos.infos.remove(staker_info_idx);
        }
        Ordering::Greater => return Err(ContractError::NotEnoughUnlocked(amount, total_amount)),
    }
    store_ve_stacker_infos(deps.storage, &info.sender, &staker_infos)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: amount.into(),
            })?,
        }))
        .add_attributes([
            attr("action", "claim_unlocked_aterra"),
            attr("depositor", info.sender),
            attr("aterra_amount", amount),
        ]))
}

pub(crate) fn compute_ve_exchange_rate(state: &State, block_height: u64) -> Decimal256 {
    let blocks_elapsed = Decimal256::from_ratio(block_height - state.last_ve_aterra_updated, 1);
    if blocks_elapsed.is_zero() {
        state.prev_ve_aterra_exchange_rate
    } else {
        state.prev_ve_aterra_exchange_rate * blocks_elapsed * state.ve_aterra_premium_rate
    }
}
