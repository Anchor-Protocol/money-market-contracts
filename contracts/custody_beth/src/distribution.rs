use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};

use crate::contract::{CLAIM_REWARDS_OPERATION, SWAP_TO_STABLE_OPERATION};
use crate::external::handle::{RewardContractExecuteMsg, RewardContractQueryMsg};
use crate::state::{read_config, BETHAccruedRewardsResponse, Config};

use moneymarket::querier::{deduct_tax, query_all_balances, query_balance};
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

/// Request withdraw reward operation to
/// reward contract and execute `distribute_hook`
/// Executor: overseer
pub fn distribute_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response<TerraMsgWrapper>> {
    let threshold = Uint128::new(1000000);
    let config: Config = read_config(deps.storage)?;

    let contract_addr = env.contract.address;

    if config.overseer_contract != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let reward_contract = deps.api.addr_humanize(&config.reward_contract)?;

    let previous_reward_balance =
        get_accrued_rewards(deps.as_ref(), reward_contract.clone(), contract_addr)?;
    if previous_reward_balance < threshold {
        return Ok(Response::default());
    }

    // Do not emit the event logs here
    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: reward_contract.to_string(),
            funds: vec![],
            msg: to_binary(&RewardContractExecuteMsg::ClaimRewards { recipient: None })?,
        }),
        CLAIM_REWARDS_OPERATION,
    )))
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
        last.id = SWAP_TO_STABLE_OPERATION;
        last.reply_on = ReplyOn::Success;
    }

    Ok(Response::new().add_submessages(messages))
}

pub(crate) fn get_accrued_rewards(
    deps: Deps,
    reward_contract_addr: Addr,
    contract_addr: Addr,
) -> StdResult<Uint128> {
    let rewards: BETHAccruedRewardsResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: reward_contract_addr.to_string(),
            msg: to_binary(&RewardContractQueryMsg::AccruedRewards {
                address: contract_addr.to_string(),
            })?,
        }))?;

    Ok(rewards.rewards)
}
