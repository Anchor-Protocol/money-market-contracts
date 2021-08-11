use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, Querier, QueryRequest, StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};

use crate::external::handle::{RewardContractHandleMsg, RewardContractQueryMsg};
use crate::state::{read_config, BETHAccruedRewardsResponse, Config};

use moneymarket::custody::HandleMsg;
use moneymarket::querier::{deduct_tax, query_all_balances, query_balance};
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

/// Request withdraw reward operation to
/// reward contract and execute `distribute_hook`
/// Executor: overseer
pub fn distribute_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult<TerraMsgWrapper> {
    let threshold = Uint128(1000000);
    let config: Config = read_config(&deps.storage)?;

    let contract_addr = env.contract.address;

    if config.overseer_contract != deps.api.canonical_address(&env.message.sender)? {
        return Err(StdError::unauthorized());
    }

    let reward_contract = deps.api.human_address(&config.reward_contract)?;

    let previous_reward_balance =
        get_accrued_rewards(deps, reward_contract.clone(), contract_addr.clone())?;
    if previous_reward_balance < threshold {
        return Ok(HandleResponse::default());
    }

    // Do not emit the event logs here
    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: reward_contract,
                send: vec![],
                msg: to_binary(&RewardContractHandleMsg::ClaimRewards { recipient: None })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.clone(),
                send: vec![],
                msg: to_binary(&HandleMsg::SwapToStableDenom {})?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                send: vec![],
                msg: to_binary(&HandleMsg::DistributeHook {})?,
            }),
        ],
        log: vec![],
        data: None,
    })
}

/// Apply swapped reward to global index
/// Executor: itself
pub fn distribute_hook<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult<TerraMsgWrapper> {
    let contract_addr = env.contract.address;
    let config: Config = read_config(&deps.storage)?;
    if env.message.sender != contract_addr {
        return Err(StdError::unauthorized());
    }

    let overseer_contract = deps.api.human_address(&config.overseer_contract)?;

    // reward_amount = (prev_balance + reward_amount) - prev_balance
    // = (0 + reward_amount) - 0 = reward_amount = balance
    let reward_amount: Uint256 =
        query_balance(deps, &contract_addr, config.stable_denom.to_string())?;
    let mut messages: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];
    if !reward_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            from_address: contract_addr,
            to_address: overseer_contract,
            amount: vec![deduct_tax(
                deps,
                Coin {
                    denom: config.stable_denom,
                    amount: reward_amount.into(),
                },
            )?],
        }));
    }

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "distribute_rewards"),
            log("buffer_rewards", reward_amount),
        ],
        data: None,
    })
}

/// Swap all coins to stable_denom
/// and execute `swap_hook`
/// Executor: itself
pub fn swap_to_stable_denom<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let contract_addr = env.contract.address;
    let balances: Vec<Coin> = query_all_balances(deps, &contract_addr)?;
    let messages: Vec<CosmosMsg<TerraMsgWrapper>> = balances
        .iter()
        .filter(|x| x.denom != config.stable_denom)
        .map(|coin: &Coin| {
            create_swap_msg(
                contract_addr.clone(),
                coin.clone(),
                config.stable_denom.clone(),
            )
        })
        .collect();

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None,
    })
}

pub(crate) fn get_accrued_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    reward_contract_addr: HumanAddr,
    contract_addr: HumanAddr,
) -> StdResult<Uint128> {
    let rewards: BETHAccruedRewardsResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: reward_contract_addr,
            msg: to_binary(&RewardContractQueryMsg::AccruedRewards {
                address: contract_addr,
            })?,
        }))?;

    Ok(rewards.rewards)
}
