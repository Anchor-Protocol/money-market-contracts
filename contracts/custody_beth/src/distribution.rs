use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, to_binary, Addr, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    ReplyOn, Response, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

use moneymarket::querier::{deduct_tax, query_all_balances, query_balance, query_token_balance};

use crate::contract::{CLAIM_REWARDS_OPERATION, SWAP_TO_STABLE_OPERATION};
use crate::error::ContractError;
use crate::external::handle::{RewardContractExecuteMsg, RewardContractQueryMsg};
use crate::state::{
    read_config, update_global_index, update_total_cumulative_rewards, BETHAccruedRewardsResponse,
    Config,
};

// REWARD_THRESHOLD
// This value is used as the minimum reward claim amount
// thus if a user's reward is less than 1 ust do not send the ClaimRewards msg
const REWARDS_THRESHOLD: Uint128 = Uint128::new(1000000);

/// Request withdraw reward operation to
/// reward contract and execute `distribute_hook`
/// Executor: overseer
pub fn distribute_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if config.overseer_contract != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let contract_addr = env.contract.address;
    let reward_contract = deps.api.addr_humanize(&config.reward_contract)?;

    let accrued_rewards =
        get_accrued_rewards(deps.as_ref(), reward_contract.clone(), contract_addr)?;
    if accrued_rewards < REWARDS_THRESHOLD {
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
pub fn distribute_hook(
    deps: DepsMut,
    env: Env,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let contract_addr = env.contract.address;
    let config: Config = read_config(deps.storage)?;

    // reward_amount = (prev_balance + reward_amount) - prev_balance
    // = (0 + reward_amount) - 0 = reward_amount = balance
    let reward_amount: Uint256 = query_balance(
        deps.as_ref(),
        contract_addr.clone(),
        config.stable_denom.to_string(),
    )?;

    let collateral_amount: Uint256 = query_token_balance(
        deps.as_ref(),
        deps.api.addr_humanize(&config.collateral_token)?,
        contract_addr,
    )?;

    update_global_index(deps.storage, &reward_amount, &collateral_amount)?;
    update_total_cumulative_rewards(deps.storage, &reward_amount)?;

    Ok(Response::new().add_attributes(vec![
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
