use std::cmp::Ordering;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use moneymarket::querier::{deduct_tax, query_balance, query_supply};

use crate::borrow::{compute_interest, compute_reward};
use crate::error::ContractError;
use crate::state::{
    read_config, read_state, read_ve_aterra_staker_infos, store_state, store_ve_stacker_infos,
    Config, State, ATERRA_NAME, VE_ATERRA_NAME,
};

pub fn bond_aterra(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    compute_interest(deps.as_ref(), &config, &mut state, env.block.height, None)?;
    compute_reward(&mut state, env.block.height);

    let bond_amount = extract_coin_amount(&info, ATERRA_NAME)?;
    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let ve_aterra_amount = bond_amount / exchange_rate;
    state.prev_aterra_supply = state.prev_aterra_supply - bond_amount;
    state.prev_ve_aterra_supply += ve_aterra_amount;
    store_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages([
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: bond_amount.into(),
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.ve_aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: info.sender.to_string(),
                    amount: ve_aterra_amount.into(),
                })?,
            }),
        ])
        .add_attributes([
            attr("action", "bond_aterra"),
            attr("depositor", info.sender),
            attr("bond_amount", bond_amount),
            attr("mint_amount", ve_aterra_amount),
        ]))
}

pub fn unbond_ve_aterra(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    compute_interest(deps.as_ref(), &config, &mut state, env.block.height, None)?;
    compute_reward(&mut state, env.block.height);

    let unbond_amount = extract_coin_amount(&info, VE_ATERRA_NAME)?;
    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let aterra_mint_amount = unbond_amount * exchange_rate;
    state.prev_aterra_supply += aterra_mint_amount;
    state.prev_ve_aterra_supply = state.prev_ve_aterra_supply - unbond_amount;
    store_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages([
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.ve_aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: unbond_amount.into(),
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: info.sender.to_string(),
                    amount: aterra_mint_amount.into(),
                })?,
            }),
        ])
        .add_attributes([
            attr("action", "unbond_ve_aterra"),
            attr("depositor", info.sender),
            attr("unbond_amount", unbond_amount),
            attr("mint_amount", aterra_mint_amount),
        ]))
}

pub fn claim_unlocked_aterra(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unlock_block_height: u64,
    amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let owner = &deps.api.addr_canonicalize(&info.sender.as_str())?;
    let mut staker_infos = read_ve_aterra_staker_infos(deps.storage, owner);

    let staker_info_idx = staker_infos
        .infos
        .iter_mut()
        .position(|x| x.unlock_block == unlock_block_height)
        .ok_or(ContractError::NoUnlockMatchingBlockHeight)?;

    if unlock_block_height > env.block.height {
        return Err(ContractError::VeStakeNotUnlocked(
            env.block.height,
            unlock_block_height,
        ));
    }

    let total_amount = staker_infos.infos[staker_info_idx].ve_aterra_qty;
    match amount.cmp(&total_amount) {
        Ordering::Less => {
            let stored_qty = &mut staker_infos.infos[staker_info_idx].ve_aterra_qty;
            *stored_qty = *stored_qty - amount;
        }
        Ordering::Equal => {
            staker_infos.infos.remove(staker_info_idx);
        }
        Ordering::Greater => return Err(ContractError::NotEnoughUnlocked(amount, total_amount)),
    }
    store_ve_stacker_infos(deps.storage, owner, &staker_infos)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: amount.into(), // how should decimal -> integer conversion happen here?
            })?,
        }))
        .add_attributes([
            attr("action", "claim_unlocked_aterra"),
            attr("depositor", info.sender),
            attr("aterra_amount", amount),
        ]))
}

pub fn deposit_stable(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Check base denom deposit
    let deposit_amount = extract_coin_amount(&info, &config.stable_denom)?;

    // Update interest related state
    let mut state: State = read_state(deps.storage)?;
    compute_interest(
        deps.as_ref(),
        &config,
        &mut state,
        env.block.height,
        Some(deposit_amount),
    )?;
    compute_reward(&mut state, env.block.height);

    // Load anchor token exchange rate with updated state
    let exchange_rate = compute_exchange_rate(
        deps.as_ref(),
        env.block.height,
        &config,
        &state,
        Some(deposit_amount),
    )?;
    let mint_amount = deposit_amount / exchange_rate;

    state.prev_aterra_supply += mint_amount;
    store_state(deps.storage, &state)?;
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: mint_amount.into(),
            })?,
        }))
        .add_attributes([
            attr("action", "deposit_stable"),
            attr("depositor", info.sender),
            attr("mint_amount", mint_amount),
            attr("deposit_amount", deposit_amount),
        ]))
}

pub fn redeem_stable(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    burn_amount: Uint128,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Update interest related state
    let mut state: State = read_state(deps.storage)?;
    compute_interest(deps.as_ref(), &config, &mut state, env.block.height, None)?;
    compute_reward(&mut state, env.block.height);

    // Load anchor token exchange rate with updated state
    let exchange_rate =
        compute_exchange_rate(deps.as_ref(), env.block.height, &config, &state, None)?;
    let redeem_amount = Uint256::from(burn_amount) * exchange_rate;

    let current_balance = query_balance(
        deps.as_ref(),
        env.contract.address,
        config.stable_denom.to_string(),
    )?;

    // Assert redeem amount
    assert_redeem_amount(&config, &state, current_balance, redeem_amount)?;

    state.prev_aterra_supply = state.prev_aterra_supply - Uint256::from(burn_amount);
    store_state(deps.storage, &state)?;
    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: burn_amount,
                })?,
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: config.stable_denom,
                        amount: redeem_amount.into(),
                    },
                )?],
            }),
        ])
        .add_attributes(vec![
            attr("action", "redeem_stable"),
            attr("burn_amount", burn_amount),
            attr("redeem_amount", redeem_amount),
        ]))
}

fn assert_redeem_amount(
    config: &Config,
    state: &State,
    current_balance: Uint256,
    redeem_amount: Uint256,
) -> Result<(), ContractError> {
    let current_balance = Decimal256::from_uint256(current_balance);
    let redeem_amount = Decimal256::from_uint256(redeem_amount);
    if redeem_amount + state.total_reserves > current_balance {
        return Err(ContractError::NoStableAvailable(
            config.stable_denom.clone(),
        ));
    }

    Ok(())
}

fn extract_coin_amount(info: &MessageInfo, coin: &str) -> Result<Uint256, ContractError> {
    let amount = info
        .funds
        .iter()
        .find(|c| c.denom == coin)
        .map(|c| Uint256::from(c.amount))
        .unwrap_or_else(Uint256::zero);

    if amount.is_zero() {
        return Err(ContractError::ZeroDeposit(coin.to_string()));
    }
    Ok(amount)
}

pub(crate) fn compute_ve_exchange_rate(state: &State, block_height: u64) -> Decimal256 {
    let blocks_elapses = Decimal256::from_ratio(block_height - state.last_ve_aterra_updated, 1);
    state.prev_ve_aterra_exchange_rate * blocks_elapses * state.ve_aterra_premium_rate
}

pub(crate) fn compute_exchange_rate(
    deps: Deps,
    block_height: u64,
    config: &Config,
    state: &State,
    deposit_amount: Option<Uint256>,
) -> StdResult<Decimal256> {
    let aterra_supply = query_supply(deps, deps.api.addr_humanize(&config.aterra_contract)?)?;
    let ve_aterra_supply = query_supply(deps, deps.api.addr_humanize(&config.ve_aterra_contract)?)?;
    let contract_balance = query_balance(
        deps,
        deps.api.addr_humanize(&config.contract_addr)?,
        config.stable_denom.to_string(),
    )? - deposit_amount.unwrap_or_else(Uint256::zero);

    Ok(compute_exchange_rate_raw(
        state,
        block_height,
        aterra_supply,
        ve_aterra_supply,
        contract_balance,
    ))
}

pub fn compute_exchange_rate_raw(
    state: &State,
    block_height: u64,
    aterra_supply: Uint256,
    ve_aterra_supply: Uint256,
    contract_balance: Uint256,
) -> Decimal256 {
    if aterra_supply.is_zero() {
        return Decimal256::one();
    }

    let ve_er = compute_ve_exchange_rate(state, block_height);
    let converted_ve = Decimal256::from_uint256(ve_aterra_supply) * ve_er;
    let effective_aterra_supply = Decimal256::from_uint256(aterra_supply) + converted_ve;

    // (aterra / stable_denom)
    // exchange_rate = (balance + total_liabilities - total_reserves) / aterra_supply
    (Decimal256::from_uint256(contract_balance) + state.total_liabilities - state.total_reserves)
        / effective_aterra_supply
}
