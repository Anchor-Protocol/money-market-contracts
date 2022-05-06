use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use moneymarket::market::ExecuteMsg;

use crate::error::ContractError;
use crate::state::{
    read_config, read_state, read_user_receipts, store_state, store_user_receipts, Config, Receipt,
    State,
};

/// Bond aterra for vterra that earns premium rate
pub fn bond(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    bond_amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let vterra_amount = bond_amount / exchange_rate;

    state.vterra_supply += vterra_amount;
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
            // mint vterra to bonder
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.vterra_contract)?.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: sender.to_string(),
                    amount: vterra_amount.into(),
                })?,
            }),
            // subtract bond amount from aterra supply stored in market contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.market_addr)?.into(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateFromVTerraActions {
                    vterra_supply: state.vterra_supply,
                    aterra_diff: moneymarket::market::Diff::Neg(bond_amount),
                    vterra_exchange_rate: exchange_rate,
                })?,
            }),
        ])
        .add_attributes([
            attr("action", "bond_aterra"),
            attr("depositor", sender),
            attr("bond_amount", bond_amount),
            attr("mint_amount", vterra_amount),
        ]))
}

pub const UNBOND_DURATION_SECS: u64 = 60 * 60 * 24 * 30;

// Unbond vterra for aterra.
// Aterra can be claimed 30 days after unbonding
pub fn unbond(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    unbond_amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    state.vterra_supply = state.vterra_supply - unbond_amount;
    store_state(deps.storage, &state)?;

    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let aterra_mint_amount = unbond_amount * exchange_rate;

    let unlock_time = env.block.time.plus_seconds(UNBOND_DURATION_SECS);
    let mut receipts = read_user_receipts(deps.storage, &sender);
    receipts.0.push_back(Receipt {
        aterra_qty: aterra_mint_amount,
        unlock_time,
    });
    store_user_receipts(deps.storage, &sender, &receipts)?;

    Ok(Response::new()
        .add_messages([
            // Burn vterra
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.vterra_contract)?.into(),
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
                msg: to_binary(&ExecuteMsg::UpdateFromVTerraActions {
                    vterra_supply: state.vterra_supply,
                    aterra_diff: moneymarket::market::Diff::Pos(aterra_mint_amount),
                    vterra_exchange_rate: exchange_rate,
                })?,
            }),
        ])
        .add_attributes([
            attr("action", "unbond_vterra"),
            attr("depositor", sender.to_string()),
            attr("unbond_amount", unbond_amount),
            attr("mint_amount", aterra_mint_amount),
            attr("unlock_time", unlock_time.to_string()),
        ]))
}

/// Rebond reverts unbonding.
/// This allows the user to decide to convert the receipts for aterra back into vterra and
/// earn the premium rate again before the receipt unlocks.
/// Rebond takes an optional amount or rebonds all receipts if not provided.
/// The receipts that have the most time remaining before they can be claimed are rebonded first.
pub fn rebond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint256>,
) -> Result<Response, ContractError> {
    let mut receipts = read_user_receipts(deps.storage, &info.sender);
    let deque = &mut receipts.0;

    let mut rebonded = Uint256::zero();
    let mut to_pop = 0;
    for receipt in deque.iter_mut().rev() {
        match amount {
            Some(amount) if receipt.aterra_qty + rebonded > amount => {
                receipt.aterra_qty = receipt.aterra_qty - (amount - rebonded);
                rebonded = amount;
            }
            _ => {
                rebonded += receipt.aterra_qty;
                to_pop += 1;
            }
        }
    }
    // remove receipts with latest unlock time
    for _ in 0..to_pop {
        deque.pop_back();
    }

    if let Some(amount) = amount {
        if rebonded < amount {
            return Err(ContractError::NotEnoughAterraReceipts(amount, rebonded));
        }
    }

    store_user_receipts(deps.storage, &info.sender, &receipts)?;

    // re-use the bonding handler
    let mut response = bond(deps, env, info.sender, rebonded)?;

    // change the action from "bond" -> "rebond"
    if let Some(idx) = response
        .attributes
        .iter()
        .position(|key| key.key.as_str() == "action")
    {
        response.attributes[idx] = Attribute::new("action", "rebond_aterra")
    }
    Ok(response)
}

/// Claim aterra having waited for lock_period (30 days) after unbonding vterra
/// If amount is specified, claim receipts in order of ascending unlock time until amount is reached
/// If amount is not specified, claim all unlocked aterra
/// A receipt is unlocked when the current block time is >= receipt.unlock_time
pub fn claim_unlocked_aterra(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint256>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut receipts = read_user_receipts(deps.storage, &info.sender);
    let deque = &mut receipts.0;

    let mut claimed = Uint256::zero();
    let mut to_pop = 0;
    for receipt in deque.iter_mut() {
        if receipt.unlock_time > env.block.time {
            continue;
        }
        match amount {
            Some(amount) if receipt.aterra_qty + claimed > amount => {
                receipt.aterra_qty = receipt.aterra_qty - (amount - claimed);
                claimed = amount;
            }
            _ => {
                claimed += receipt.aterra_qty;
                to_pop += 1;
            }
        }
    }
    // remove receipts with earliest unlock time
    for _ in 0..to_pop {
        deque.pop_front();
    }

    if let Some(amount) = amount {
        if claimed < amount {
            return Err(ContractError::NotEnoughUnlocked(amount, claimed));
        }
    }

    store_user_receipts(deps.storage, &info.sender, &receipts)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.into(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: claimed.into(),
            })?,
        }))
        .add_attributes([
            attr("action", "claim_unlocked_aterra"),
            attr("depositor", info.sender),
            attr("aterra_amount", claimed),
        ]))
}

/// Exchange rate of aterra / vterra
/// ex: 1 ve * ER => ER aterra
pub(crate) fn compute_ve_exchange_rate(state: &State, block_height: u64) -> Decimal256 {
    moneymarket::vterra::compute_ve_exchange_rate(
        state.prev_epoch_vterra_exchange_rate,
        state.premium_rate,
        state.last_updated,
        block_height,
    )
}
