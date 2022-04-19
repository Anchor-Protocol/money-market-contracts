use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use moneymarket::market::ExecuteMsg;

use crate::error::ContractError;
use crate::state::{
    read_config, read_state, read_user_receipts, store_state, store_user_receipts, Config, Receipt,
    State,
};

/// Bond aterra for ve aterra that earns premium rate
pub fn bond(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    bond_amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let ve_aterra_amount = bond_amount / exchange_rate;

    state.ve_aterra_supply += ve_aterra_amount;
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
                msg: to_binary(&ExecuteMsg::UpdateFromVeActions {
                    ve_aterra_supply: state.ve_aterra_supply,
                    aterra_diff: moneymarket::market::Diff::Neg(bond_amount),
                    ve_exchange_rate: exchange_rate,
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

pub const UNBOND_DURATION_SECS: u64 = 60 * 60 * 24 * 30;

// Unbond ve aterra for aterra.
// Aterra can be claimed 30 days after unbonding
pub fn unbond(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    unbond_amount: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    state.ve_aterra_supply = state.ve_aterra_supply - unbond_amount;
    store_state(deps.storage, &state)?;

    let exchange_rate = compute_ve_exchange_rate(&state, env.block.height);

    let aterra_mint_amount = unbond_amount * exchange_rate;

    let mut staker_infos = read_user_receipts(deps.storage, &sender);
    let unlock_time = env.block.time.plus_seconds(UNBOND_DURATION_SECS);
    staker_infos.infos.push_back(Receipt {
        aterra_qty: aterra_mint_amount,
        unlock_time,
    });
    store_user_receipts(deps.storage, &sender, &staker_infos)?;

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
                msg: to_binary(&ExecuteMsg::UpdateFromVeActions {
                    ve_aterra_supply: state.ve_aterra_supply,
                    aterra_diff: moneymarket::market::Diff::Pos(aterra_mint_amount),
                    ve_exchange_rate: exchange_rate,
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

/// Claim aterra having waited for lock_period (30 days) after unbonding ve aterra
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

    let mut unlock_infos = read_user_receipts(deps.storage, &info.sender);
    let queue = &mut unlock_infos.infos;

    let mut claimed = Uint256::zero();
    let mut to_pop = 0;
    for idx in 0..queue.len() {
        let mut receipt = queue.get_mut(idx).unwrap();
        if receipt.unlock_time > env.block.time {
            break;
        }
        match amount {
            Some(amount) => {
                // if this receipt won't fill amount => take all and remove receipt
                if amount - claimed >= receipt.aterra_qty {
                    claimed += receipt.aterra_qty;
                    to_pop += 1;
                    continue;
                }
                // this receipt has more than the requested amount => take whats needed and update receipt
                receipt.aterra_qty = receipt.aterra_qty - (amount - claimed);
                claimed = amount;
            }
            None => {
                // remove all unlocked receipts
                claimed += receipt.aterra_qty;
                to_pop += 1;
            }
        }
    }
    for _ in 0..to_pop {
        queue.pop_front();
    }

    if let Some(amount) = amount {
        if claimed < amount {
            return Err(ContractError::NotEnoughUnlocked(amount, claimed));
        }
    }

    store_user_receipts(deps.storage, &info.sender, &unlock_infos)?;

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

pub(crate) fn compute_ve_exchange_rate(state: &State, block_height: u64) -> Decimal256 {
    moneymarket::ve_aterra::compute_ve_exchange_rate(
        state.prev_epoch_ve_aterra_exchange_rate,
        state.premium_rate,
        state.last_updated,
        block_height,
    )
}
