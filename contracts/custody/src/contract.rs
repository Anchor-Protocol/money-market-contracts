use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Env, Extern, HandleResult, InitResponse, InitResult,
    Querier, StdError, StdResult, Storage,
};

use crate::collateral::{
    deposit_collateral, liquidate_collateral, lock_collateral, query_borrower, query_borrowers,
    unlock_collateral, withdraw_collateral,
};
use crate::distribution::{distribute_hook, distribute_rewards, swap_to_stable_denom};
use crate::msg::{ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg};
use crate::state::{read_config, store_config, Config};

use cw20::Cw20ReceiveMsg;
use terra_cosmwasm::TerraMsgWrapper;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> InitResult {
    let config = Config {
        overseer_contract: deps.api.canonical_address(&msg.overseer_contract)?,
        collateral_token: deps.api.canonical_address(&msg.collateral_token)?,
        market_contract: deps.api.canonical_address(&msg.market_contract)?,
        reward_contract: deps.api.canonical_address(&msg.reward_contract)?,
        terraswap_contract: deps.api.canonical_address(&msg.terraswap_contract)?,
        stable_denom: msg.stable_denom,
    };

    store_config(&mut deps.storage, &config)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult<TerraMsgWrapper> {
    match msg {
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::LockCollateral { borrower, amount } => {
            lock_collateral(deps, env, borrower, amount)
        }
        HandleMsg::UnlockCollateral { borrower, amount } => {
            unlock_collateral(deps, env, borrower, amount)
        }
        HandleMsg::DistributeRewards {} => distribute_rewards(deps, env),
        HandleMsg::DistributeHook {} => distribute_hook(deps, env),
        HandleMsg::SwapToRewardDenom {} => swap_to_stable_denom(deps, env),
        HandleMsg::WithdrawCollateral { amount } => withdraw_collateral(deps, env, amount),
        HandleMsg::LiquidateCollateral { borrower, amount } => {
            liquidate_collateral(deps, env, borrower, amount)
        }
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult<TerraMsgWrapper> {
    let contract_addr = env.message.sender;
    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::DepositCollateral {} => {
                // only asset contract can execute this message
                let config: Config = read_config(&deps.storage)?;
                if deps.api.canonical_address(&contract_addr)? != config.collateral_token {
                    return Err(StdError::unauthorized());
                }

                deposit_collateral(deps, cw20_msg.sender, cw20_msg.amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Borrower { address } => to_binary(&query_borrower(deps, address)?),
        QueryMsg::Borrowers { start_after, limit } => {
            to_binary(&query_borrowers(deps, start_after, limit)?)
        }
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config: Config = read_config(&deps.storage)?;
    Ok(ConfigResponse {
        collateral_token: deps.api.human_address(&config.collateral_token)?,
        overseer_contract: deps.api.human_address(&config.overseer_contract)?,
        market_contract: deps.api.human_address(&config.market_contract)?,
        reward_contract: deps.api.human_address(&config.reward_contract)?,
        terraswap_contract: deps.api.human_address(&config.terraswap_contract)?,
        stable_denom: config.stable_denom,
    })
}
