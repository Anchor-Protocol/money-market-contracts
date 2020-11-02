use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse,
    HandleResult, Querier, StdError, Storage, Uint128, WasmMsg,
};

use crate::external::handle::RewardContractHandleMsg;
use crate::msg::HandleMsg;
use crate::state::{
    increase_global_index, read_borrower_info, read_config, read_global_index,
    remove_borrower_info, store_borrower_info, BorrowerInfo, Config,
};
use moneymarket::{
    deduct_tax, load_all_balances, load_balance, load_distribution_params, load_token_balance,
    DistributionParamsResponse,
};
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

/// Request withdraw reward operation to
/// reward contract and execute `distribute_hook`
/// Executor: anyone
pub fn distribute_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;
    let reward_contract = deps.api.human_address(&config.reward_contract)?;
    let prev_balance = load_balance(&deps, &env.contract.address, config.reward_denom)?;
    let contract_addr = env.contract.address;

    // Do not emit the event logs here
    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: reward_contract,
                send: vec![],
                msg: to_binary(&RewardContractHandleMsg::WithdrawReward {})?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.clone(),
                send: vec![],
                msg: to_binary(&HandleMsg::SwapToRewardDenom {})?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                send: vec![],
                msg: to_binary(&HandleMsg::DistributeHook { prev_balance })?,
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
    prev_balance: Uint128,
) -> HandleResult<TerraMsgWrapper> {
    let contract_addr = env.contract.address;
    let config: Config = read_config(&deps.storage)?;
    if env.message.sender != contract_addr {
        return Err(StdError::unauthorized());
    }

    let overseer_contract = deps.api.human_address(&config.overseer_contract)?;
    let collateral_token = deps.api.human_address(&config.collateral_token)?;

    // reward_amount = (prev_balance + reward_amount) - prev_balance
    let cur_balance: Uint128 =
        load_balance(&deps, &contract_addr, config.reward_denom.to_string())?;
    let reward_amount = (cur_balance - prev_balance).unwrap();
    // load distribution params from the overseer contract
    let distribution_params: DistributionParamsResponse =
        load_distribution_params(&deps, &overseer_contract, &collateral_token)?;

    // load total bAsset balance
    let total_balance = load_token_balance(&deps, &collateral_token, &contract_addr)?;

    let depositor_subsidy = reward_amount * distribution_params.a_value;
    let borrower_plus_buffer_rewards = (reward_amount - depositor_subsidy).unwrap();
    let buffer_rewards = borrower_plus_buffer_rewards * distribution_params.buffer_tax_rate;
    let borrower_rewards = (borrower_plus_buffer_rewards - buffer_rewards).unwrap();

    increase_global_index(
        &mut deps.storage,
        Decimal::from_ratio(borrower_rewards, total_balance),
    )?;

    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: contract_addr.clone(),
                to_address: deps.api.human_address(&config.market_contract)?,
                amount: vec![deduct_tax(
                    deps,
                    Coin {
                        denom: config.reward_denom.to_string(),
                        amount: depositor_subsidy,
                    },
                )?],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: contract_addr,
                to_address: overseer_contract,
                amount: vec![deduct_tax(
                    deps,
                    Coin {
                        denom: config.reward_denom,
                        amount: buffer_rewards,
                    },
                )?],
            }),
        ],
        log: vec![
            log("action", "distribute_rewards"),
            log("borrower_rewards", borrower_rewards),
            log("buffer_rewards", buffer_rewards),
            log("depositer_subsidy", depositor_subsidy),
        ],
        data: None,
    })
}

/// Swap all coins to reward_denom
/// and execute `swap_hook`
/// Executor: itself
pub fn swap_to_reward_denom<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult<TerraMsgWrapper> {
    let config: Config = read_config(&deps.storage)?;
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let contract_addr = env.contract.address;
    let balances: Vec<Coin> = load_all_balances(&deps, &contract_addr)?;
    let messages: Vec<CosmosMsg<TerraMsgWrapper>> = balances
        .iter()
        .filter(|x| x.denom != config.reward_denom)
        .map(|coin: &Coin| {
            create_swap_msg(
                contract_addr.clone(),
                coin.clone(),
                config.reward_denom.clone(),
            )
        })
        .collect();

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None,
    })
}

/// Claim collected reward from bAsset
/// Executor: borrower
pub fn claim_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult<TerraMsgWrapper> {
    let contract_addr = env.contract.address;
    let borrower = env.message.sender;
    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let mut borrower_info: BorrowerInfo = read_borrower_info(&deps.storage, &borrower_raw);
    let config: Config = read_config(&deps.storage)?;

    // Compute current reward + pending_reward
    let global_index: Decimal = read_global_index(&deps.storage);
    let reward_amount = (borrower_info.balance * global_index
        - borrower_info.balance * borrower_info.reward_index)?
        + borrower_info.pending_reward;

    // Update reward_index to global_index
    borrower_info.reward_index = global_index;
    borrower_info.pending_reward = Uint128::zero();

    if borrower_info.balance == Uint128::zero() && borrower_info.pending_reward == Uint128::zero() {
        remove_borrower_info(&mut deps.storage, &borrower_raw);
    } else {
        store_borrower_info(&mut deps.storage, &borrower_raw, &borrower_info)?;
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: contract_addr,
            to_address: borrower.clone(),
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: config.reward_denom,
                    amount: reward_amount,
                },
            )?],
        })],
        log: vec![
            log("action", "claim_reward"),
            log("borrower", borrower.as_str()),
            log("reward_amount", reward_amount),
        ],
        data: None,
    })
}
