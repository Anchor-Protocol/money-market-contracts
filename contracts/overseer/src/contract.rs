use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, InitResponse, InitResult, Querier, StdError, StdResult, Storage,
    WasmMsg,
};

use crate::collateral::{
    liquidate_collateral, lock_collateral, query_all_collaterals, query_borrow_limit,
    query_collaterals, unlock_collateral,
};
use crate::msg::{
    ConfigResponse, DistributionParamsResponse, HandleMsg, InitMsg, QueryMsg, WhitelistResponse,
    WhitelistResponseElem,
};
use crate::state::{
    read_config, read_epoch_state, read_whitelist, read_whitelist_elem, store_config,
    store_epoch_state, store_whitelist_elem, Config, EpochState, WhitelistElem,
};

use moneymarket::{
    deduct_tax, query_balance, query_epoch_state, CustodyHandleMsg, EpochStateResponse,
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    store_config(
        &mut deps.storage,
        &Config {
            owner_addr: deps.api.canonical_address(&msg.owner_addr)?,
            oracle_contract: deps.api.canonical_address(&msg.oracle_contract)?,
            market_contract: deps.api.canonical_address(&msg.market_contract)?,
            liquidation_model: deps.api.canonical_address(&msg.liquidation_model)?,
            stable_denom: msg.stable_denom,
            epoch_period: msg.epoch_period,
            distribution_threshold: msg.distribution_threshold,
            target_deposit_rate: msg.target_deposit_rate,
            buffer_distribution_rate: msg.buffer_distribution_rate,
        },
    )?;

    store_epoch_state(
        &mut deps.storage,
        &EpochState {
            deposit_rate: Decimal256::zero(),
            prev_a_token_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
            last_executed_height: env.block.height,
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::UpdateConfig {
            owner_addr,
            oracle_contract,
            liquidation_model,
            distribution_threshold,
            target_deposit_rate,
            buffer_distribution_rate,
            epoch_period,
        } => update_config(
            deps,
            env,
            owner_addr,
            oracle_contract,
            liquidation_model,
            distribution_threshold,
            target_deposit_rate,
            buffer_distribution_rate,
            epoch_period,
        ),
        HandleMsg::Whitelist {
            collateral_token,
            custody_contract,
            ltv,
        } => register_whitelist(deps, env, collateral_token, custody_contract, ltv),
        HandleMsg::UpdateWhitelist {
            collateral_token,
            custody_contract,
            ltv,
        } => update_whitelist(deps, env, collateral_token, custody_contract, ltv),
        HandleMsg::ExecuteEpochOperations {} => execute_epoch_operations(deps, env),
        HandleMsg::LockCollateral { collaterals } => lock_collateral(deps, env, collaterals),
        HandleMsg::UnlockCollateral { collaterals } => unlock_collateral(deps, env, collaterals),
        HandleMsg::LiquidateCollateral { borrower } => liquidate_collateral(deps, env, borrower),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner_addr: Option<HumanAddr>,
    oracle_contract: Option<HumanAddr>,
    liquidation_model: Option<HumanAddr>,
    distribution_threshold: Option<Decimal256>,
    target_deposit_rate: Option<Decimal256>,
    buffer_distribution_rate: Option<Decimal256>,
    epoch_period: Option<u64>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;

    if deps.api.canonical_address(&env.message.sender)? != config.owner_addr {
        return Err(StdError::unauthorized());
    }

    if let Some(owner_addr) = owner_addr {
        config.owner_addr = deps.api.canonical_address(&owner_addr)?;
    }

    if let Some(oracle_contract) = oracle_contract {
        config.oracle_contract = deps.api.canonical_address(&oracle_contract)?;
    }

    if let Some(liquidation_model) = liquidation_model {
        config.liquidation_model = deps.api.canonical_address(&liquidation_model)?;
    }

    if let Some(distribution_threshold) = distribution_threshold {
        config.distribution_threshold = distribution_threshold;
    }

    if let Some(buffer_distribution_rate) = buffer_distribution_rate {
        config.buffer_distribution_rate = buffer_distribution_rate;
    }

    if let Some(target_deposit_rate) = target_deposit_rate {
        config.target_deposit_rate = target_deposit_rate;
    }

    if let Some(epoch_period) = epoch_period {
        config.epoch_period = epoch_period;
    }

    store_config(&mut deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}

pub fn register_whitelist<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    custody_contract: HumanAddr,
    ltv: Decimal256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner_addr {
        return Err(StdError::unauthorized());
    }

    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    if read_whitelist_elem(&deps.storage, &collateral_token_raw).is_ok() {
        return Err(StdError::generic_err(
            "The collateral token was already registered",
        ));
    }

    store_whitelist_elem(
        &mut deps.storage,
        &collateral_token_raw,
        &WhitelistElem {
            custody_contract: deps.api.canonical_address(&custody_contract)?,
            ltv,
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "register_whitelist"),
            log("collateral_token", collateral_token),
            log("custody_contract", custody_contract),
            log("LTV", ltv),
        ],
        data: None,
    })
}

pub fn update_whitelist<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    custody_contract: Option<HumanAddr>,
    ltv: Option<Decimal256>,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner_addr {
        return Err(StdError::unauthorized());
    }

    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    let mut whitelist_elem: WhitelistElem =
        read_whitelist_elem(&deps.storage, &collateral_token_raw)?;

    if let Some(custody_contract) = custody_contract {
        whitelist_elem.custody_contract = deps.api.canonical_address(&custody_contract)?;
    }

    if let Some(ltv) = ltv {
        whitelist_elem.ltv = ltv;
    }

    store_whitelist_elem(&mut deps.storage, &collateral_token_raw, &whitelist_elem)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "update_whitelist"),
            log("collateral_token", collateral_token),
            log(
                "custody_contract",
                deps.api.human_address(&whitelist_elem.custody_contract)?,
            ),
            log("LTV", whitelist_elem.ltv),
        ],
        data: None,
    })
}

pub fn execute_epoch_operations<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let state: EpochState = read_epoch_state(&deps.storage)?;
    if env.block.height < state.last_executed_height + config.epoch_period {
        return Err(StdError::generic_err("Epoch period is not passed"));
    }

    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - state.last_executed_height);

    // Compute next epoch state
    let market_contract: HumanAddr = deps.api.human_address(&config.market_contract)?;
    let epoch_state: EpochStateResponse = query_epoch_state(&deps, &market_contract)?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate = epoch_state.exchange_rate / state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    let mut messages: Vec<CosmosMsg> = vec![];

    // Distribute Interest Buffer to depositor
    // Only executed when deposit rate < distribution_threshold
    let mut distributed_interest: Uint256 = Uint256::zero();
    if deposit_rate < config.distribution_threshold {
        // missing_deposit_rate(_per_block)
        let missing_deposit_rate = config.distribution_threshold - deposit_rate;
        let prev_deposits = state.prev_a_token_supply * state.prev_exchange_rate;

        // missing_deposits = prev_deposits * missing_deposit_rate(_per_block) * blocks
        let missing_deposits = prev_deposits * blocks * missing_deposit_rate;
        let interest_buffer = query_balance(
            &deps,
            &env.contract.address,
            config.stable_denom.to_string(),
        )?;
        let distribution_buffer = interest_buffer * config.buffer_distribution_rate;

        // When there was not enough deposits happens,
        // distribute interest to market contract
        distributed_interest = std::cmp::min(missing_deposits, distribution_buffer);

        if !distributed_interest.is_zero() {
            // Send some portion of interest buffer to Market contract
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address,
                to_address: deps.api.human_address(&config.market_contract)?,
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: config.stable_denom,
                        amount: distributed_interest.into(),
                    },
                )?],
            }));
        }
    }

    // Execute DistributeRewards
    let whitelist: Vec<WhitelistResponseElem> = read_whitelist(&deps, None, None)?;
    for elem in whitelist.iter() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: elem.custody_contract.clone(),
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::DistributeRewards {})?,
        }));
    }

    // update last_executed_height
    store_epoch_state(
        &mut deps.storage,
        &EpochState {
            last_executed_height: env.block.height,
            prev_exchange_rate: epoch_state.exchange_rate,
            prev_a_token_supply: epoch_state.a_token_supply,
            deposit_rate,
        },
    )?;

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "epoch_operations"),
            log("distributed_interest", distributed_interest),
            log("deposit_rate", deposit_rate),
            log("exchange_rate", epoch_state.exchange_rate),
            log("a_token_supply", epoch_state.a_token_supply),
        ],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::EpochState {} => to_binary(&query_state(deps)?),
        QueryMsg::Whitelist {
            collateral_token,
            start_after,
            limit,
        } => to_binary(&query_whitelist(
            deps,
            collateral_token,
            start_after,
            limit,
        )?),
        QueryMsg::Collaterals { borrower } => to_binary(&query_collaterals(deps, borrower)?),
        QueryMsg::AllCollaterals { start_after, limit } => {
            to_binary(&query_all_collaterals(deps, start_after, limit)?)
        }
        QueryMsg::DistributionParams {} => to_binary(&query_distribution_params(deps)?),
        QueryMsg::BorrowLimit { borrower } => to_binary(&query_borrow_limit(deps, borrower)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config: Config = read_config(&deps.storage)?;
    Ok(ConfigResponse {
        owner_addr: deps.api.human_address(&config.owner_addr)?,
        oracle_contract: deps.api.human_address(&config.oracle_contract)?,
        market_contract: deps.api.human_address(&config.market_contract)?,
        liquidation_model: deps.api.human_address(&config.liquidation_model)?,
        stable_denom: config.stable_denom,
        epoch_period: config.epoch_period,
        distribution_threshold: config.distribution_threshold,
        target_deposit_rate: config.target_deposit_rate,
        buffer_distribution_rate: config.buffer_distribution_rate,
    })
}

pub fn query_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<EpochState> {
    read_epoch_state(&deps.storage)
}

pub fn query_whitelist<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    collateral_token: Option<HumanAddr>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<WhitelistResponse> {
    if let Some(collateral_token) = collateral_token {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(
            &deps.storage,
            &deps.api.canonical_address(&collateral_token)?,
        )?;
        Ok(WhitelistResponse {
            elems: vec![WhitelistResponseElem {
                ltv: whitelist_elem.ltv,
                custody_contract: deps.api.human_address(&whitelist_elem.custody_contract)?,
                collateral_token,
            }],
        })
    } else {
        let start_after = if let Some(start_after) = start_after {
            Some(deps.api.canonical_address(&start_after)?)
        } else {
            None
        };

        let whitelist: Vec<WhitelistResponseElem> = read_whitelist(&deps, start_after, limit)?;
        Ok(WhitelistResponse { elems: whitelist })
    }
}

pub fn query_distribution_params<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<DistributionParamsResponse> {
    let config: Config = read_config(&deps.storage)?;
    let epoch_state: EpochState = read_epoch_state(&deps.storage)?;

    Ok(DistributionParamsResponse {
        target_deposit_rate: config.target_deposit_rate,
        deposit_rate: epoch_state.deposit_rate,
        distribution_threshold: config.distribution_threshold,
    })
}
