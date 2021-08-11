use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, ReplyOn, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};

use crate::external::handle::RewardContractExecuteMsg;
use crate::state::{read_config, Config};

use moneymarket::querier::{deduct_tax, query_all_balances, query_balance};
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

/// Request withdraw reward operation to
/// reward contract and execute `distribute_hook`
/// Executor: overseer
pub fn distribute_rewards(
    deps: DepsMut,
    info: MessageInfo,
) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;
    if config.overseer_contract != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let reward_contract = deps.api.addr_humanize(&config.reward_contract)?;

    // Do not emit the event logs here
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: reward_contract.to_string(),
                funds: vec![],
                msg: to_binary(&RewardContractExecuteMsg::ClaimRewards { recipient: None })?,
            }),
            1,
        )]),
    )
}

/// Apply swapped reward to global index
/// Executor: itself
pub fn distribute_hook(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let contract_addr = env.contract.address;
    let config: Config = read_config(deps.storage)?;
    let overseer_contract = deps.api.addr_humanize(&config.overseer_contract)?;

    // reward_amount = (prev_balance + reward_amount) - prev_balance
    // = (0 + reward_amount) - 0 = reward_amount = balance
    let reward_amount: Uint256 = query_balance(
        deps.as_ref(),
        contract_addr,
        config.stable_denom.to_string(),
    )?;
    let mut messages: Vec<SubMsg<TerraMsgWrapper>> = vec![];
    if !reward_amount.is_zero() {
        messages.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: overseer_contract.to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom,
                    amount: reward_amount.into(),
                },
            )?],
        })));
    }

    Ok(Response::new()
        .add_submessages(messages)
        .add_attributes(vec![
            attr("action", "distribute_rewards"),
            attr("buffer_rewards", reward_amount),
        ]))
}

/// Swap all coins to stable_denom
/// and execute `swap_hook`
/// Executor: itself
pub fn swap_to_stable_denom(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let config: Config = read_config(deps.storage)?;

    let contract_addr = env.contract.address;
    let balances: Vec<Coin> = query_all_balances(deps.as_ref(), contract_addr)?;
    let mut messages: Vec<SubMsg<TerraMsgWrapper>> = balances
        .iter()
        .filter(|x| x.denom != config.stable_denom)
        .map(|coin: &Coin| SubMsg::new(create_swap_msg(coin.clone(), config.stable_denom.clone())))
        .collect();

    if let Some(last) = messages.last_mut() {
        last.id = 2;
        last.reply_on = ReplyOn::Success;
    }

    Ok(Response::new().add_submessages(messages))
}
