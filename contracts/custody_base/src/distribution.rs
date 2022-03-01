use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{attr, BankMsg, Coin, CosmosMsg, DepsMut, Env, ReplyOn, Response, SubMsg};

use crate::contract::SWAP_TO_STABLE_OPERATION;
use crate::error::ContractError;
use crate::state::{read_config, Config};

use moneymarket::querier::{deduct_tax, query_all_balances, query_balance};
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

/// Apply swapped reward to global index
/// Executor: itself
pub fn distribute_hook(
    deps: DepsMut,
    env: Env,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
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
    let mut messages: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];
    if !reward_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: overseer_contract.to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom,
                    amount: reward_amount.into(),
                },
            )?],
        }));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "distribute_rewards"),
        attr("buffer_rewards", reward_amount),
    ]))
}

/// Swap all coins to stable_denom
/// and execute `swap_hook`
/// Executor: itself
pub fn swap_to_stable_denom(
    deps: DepsMut,
    env: Env,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let contract_addr = env.contract.address;
    let balances: Vec<Coin> = query_all_balances(deps.as_ref(), contract_addr)?;
    let mut messages: Vec<SubMsg<TerraMsgWrapper>> = balances
        .iter()
        .filter(|x| x.denom != config.stable_denom)
        .map(|coin: &Coin| SubMsg::new(create_swap_msg(coin.clone(), config.stable_denom.clone())))
        .collect();

    if let Some(last) = messages.last_mut() {
        last.id = SWAP_TO_STABLE_OPERATION;
        last.reply_on = ReplyOn::Success;
    }

    Ok(Response::new().add_submessages(messages))
}
