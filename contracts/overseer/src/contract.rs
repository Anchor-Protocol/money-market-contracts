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
use crate::querier::query_epoch_state;
use crate::state::{
    read_config, read_epoch_state, read_whitelist, read_whitelist_elem, store_config,
    store_epoch_state, store_whitelist_elem, Config, EpochState, WhitelistElem,
};

use moneymarket::custody::HandleMsg as CustodyHandleMsg;
use moneymarket::market::EpochStateResponse;
use moneymarket::market::HandleMsg as MarketHandleMsg;
use moneymarket::overseer::{
    ConfigResponse, DistributionParamsResponse, HandleMsg, InitMsg, QueryMsg, WhitelistResponse,
    WhitelistResponseElem,
};
use moneymarket::querier::{deduct_tax, query_balance};

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
            liquidation_contract: deps.api.canonical_address(&msg.liquidation_contract)?,
            collector_contract: deps.api.canonical_address(&msg.collector_contract)?,
            stable_denom: msg.stable_denom,
            epoch_period: msg.epoch_period,
            threshold_deposit_rate: msg.threshold_deposit_rate,
            target_deposit_rate: msg.target_deposit_rate,
            buffer_distribution_factor: msg.buffer_distribution_factor,
            anc_purchase_factor: msg.anc_purchase_factor,
            price_timeframe: msg.price_timeframe,
        },
    )?;

    store_epoch_state(
        &mut deps.storage,
        &EpochState {
            deposit_rate: Decimal256::zero(),
            prev_aterra_supply: Uint256::zero(),
            prev_interest_buffer: Uint256::zero(),
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
            liquidation_contract,
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
            anc_purchase_factor,
            epoch_period,
            price_timeframe,
        } => update_config(
            deps,
            env,
            owner_addr,
            oracle_contract,
            liquidation_contract,
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
            anc_purchase_factor,
            epoch_period,
            price_timeframe,
        ),
        HandleMsg::Whitelist {
            name,
            symbol,
            collateral_token,
            custody_contract,
            max_ltv,
        } => register_whitelist(
            deps,
            env,
            name,
            symbol,
            collateral_token,
            custody_contract,
            max_ltv,
        ),
        HandleMsg::UpdateWhitelist {
            collateral_token,
            custody_contract,
            max_ltv,
        } => update_whitelist(deps, env, collateral_token, custody_contract, max_ltv),
        HandleMsg::ExecuteEpochOperations {} => execute_epoch_operations(deps, env),
        HandleMsg::UpdateEpochState { interest_buffer } => {
            update_epoch_state(deps, env, interest_buffer)
        }
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
    liquidation_contract: Option<HumanAddr>,
    threshold_deposit_rate: Option<Decimal256>,
    target_deposit_rate: Option<Decimal256>,
    buffer_distribution_factor: Option<Decimal256>,
    anc_purchase_factor: Option<Decimal256>,
    epoch_period: Option<u64>,
    price_timeframe: Option<u64>,
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

    if let Some(liquidation_contract) = liquidation_contract {
        config.liquidation_contract = deps.api.canonical_address(&liquidation_contract)?;
    }

    if let Some(threshold_deposit_rate) = threshold_deposit_rate {
        config.threshold_deposit_rate = threshold_deposit_rate;
    }

    if let Some(buffer_distribution_factor) = buffer_distribution_factor {
        config.buffer_distribution_factor = buffer_distribution_factor;
    }

    if let Some(anc_purchase_factor) = anc_purchase_factor {
        config.anc_purchase_factor = anc_purchase_factor;
    }

    if let Some(target_deposit_rate) = target_deposit_rate {
        config.target_deposit_rate = target_deposit_rate;
    }

    if let Some(epoch_period) = epoch_period {
        config.epoch_period = epoch_period;
    }

    if let Some(price_timeframe) = price_timeframe {
        config.price_timeframe = price_timeframe;
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
    name: String,
    symbol: String,
    collateral_token: HumanAddr,
    custody_contract: HumanAddr,
    max_ltv: Decimal256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner_addr {
        return Err(StdError::unauthorized());
    }

    let collateral_token_raw = deps.api.canonical_address(&collateral_token)?;
    if read_whitelist_elem(&deps.storage, &collateral_token_raw).is_ok() {
        return Err(StdError::generic_err(
            "Token is already registered as collateral",
        ));
    }

    store_whitelist_elem(
        &mut deps.storage,
        &collateral_token_raw,
        &WhitelistElem {
            name: name.to_string(),
            symbol: symbol.to_string(),
            custody_contract: deps.api.canonical_address(&custody_contract)?,
            max_ltv,
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "register_whitelist"),
            log("name", name),
            log("symbol", symbol),
            log("collateral_token", collateral_token),
            log("custody_contract", custody_contract),
            log("LTV", max_ltv),
        ],
        data: None,
    })
}

pub fn update_whitelist<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collateral_token: HumanAddr,
    custody_contract: Option<HumanAddr>,
    max_ltv: Option<Decimal256>,
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

    if let Some(max_ltv) = max_ltv {
        whitelist_elem.max_ltv = max_ltv;
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
            log("LTV", whitelist_elem.max_ltv),
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
        return Err(StdError::generic_err(format!(
            "An epoch has not passed yet; last executed height: {}",
            state.last_executed_height
        )));
    }

    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - state.last_executed_height);

    // Compute next epoch state
    let market_contract: HumanAddr = deps.api.human_address(&config.market_contract)?;
    let epoch_state: EpochStateResponse =
        query_epoch_state(&deps, &market_contract, env.block.height)?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate = epoch_state.exchange_rate / state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut interest_buffer = query_balance(
        &deps,
        &env.contract.address,
        config.stable_denom.to_string(),
    )?;

    // Send accrued_buffer * config.anc_purchase_factor amount stable token to collector
    let accrued_buffer = interest_buffer - state.prev_interest_buffer;
    let anc_purchase_amount = accrued_buffer * config.anc_purchase_factor;
    if !anc_purchase_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: deps.api.human_address(&config.collector_contract)?,
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: config.stable_denom.to_string(),
                    amount: anc_purchase_amount.into(),
                },
            )?],
        }));
    }

    // Deduct anc_purchase_amount from the interest_buffer
    interest_buffer = interest_buffer - anc_purchase_amount;

    // Distribute Interest Buffer to depositor
    // Only executed when deposit rate < threshold_deposit_rate
    let mut distributed_interest: Uint256 = Uint256::zero();
    if deposit_rate < config.threshold_deposit_rate {
        // missing_deposit_rate(_per_block)
        let missing_deposit_rate = config.threshold_deposit_rate - deposit_rate;
        let prev_deposits = state.prev_aterra_supply * state.prev_exchange_rate;

        // missing_deposits = prev_deposits * missing_deposit_rate(_per_block) * blocks
        let missing_deposits = prev_deposits * blocks * missing_deposit_rate;
        let distribution_buffer = interest_buffer * config.buffer_distribution_factor;

        // When there was not enough deposits happens,
        // distribute interest to market contract
        distributed_interest = std::cmp::min(missing_deposits, distribution_buffer);
        interest_buffer = interest_buffer - distributed_interest;

        if !distributed_interest.is_zero() {
            // Send some portion of interest buffer to Market contract
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: market_contract,
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

    // Execute store epoch state operation
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address,
        send: vec![],
        msg: to_binary(&HandleMsg::UpdateEpochState { interest_buffer })?,
    }));

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "epoch_operations"),
            log("deposit_rate", deposit_rate),
            log("exchange_rate", epoch_state.exchange_rate),
            log("aterra_supply", epoch_state.aterra_supply),
            log("distributed_interest", distributed_interest),
            log("anc_purchase_amount", anc_purchase_amount),
        ],
        data: None,
    })
}

pub fn update_epoch_state<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    // To store interest buffer before receiving epoch staking rewards,
    // pass interest_buffer from execute_epoch_operations
    interest_buffer: Uint256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let state: EpochState = read_epoch_state(&deps.storage)?;
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - state.last_executed_height);

    // Compute next epoch state
    let market_contract: HumanAddr = deps.api.human_address(&config.market_contract)?;
    let epoch_state: EpochStateResponse =
        query_epoch_state(&deps, &market_contract, env.block.height)?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate = epoch_state.exchange_rate / state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    // store updated epoch state
    store_epoch_state(
        &mut deps.storage,
        &EpochState {
            last_executed_height: env.block.height,
            prev_aterra_supply: epoch_state.aterra_supply,
            prev_exchange_rate: epoch_state.exchange_rate,
            prev_interest_buffer: interest_buffer,
            deposit_rate,
        },
    )?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market_contract,
            send: vec![],
            msg: to_binary(&MarketHandleMsg::ExecuteEpochOperations {
                target_deposit_rate: config.target_deposit_rate,
                deposit_rate,
            })?,
        })],
        log: vec![
            log("action", "update_epoch_state"),
            log("deposit_rate", deposit_rate),
            log("aterra_supply", epoch_state.aterra_supply),
            log("exchange_rate", epoch_state.exchange_rate),
            log("interest_buffer", interest_buffer),
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
        QueryMsg::BorrowLimit {
            borrower,
            block_time,
        } => to_binary(&query_borrow_limit(deps, borrower, block_time)?),
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
        liquidation_contract: deps.api.human_address(&config.liquidation_contract)?,
        collector_contract: deps.api.human_address(&config.collector_contract)?,
        stable_denom: config.stable_denom,
        epoch_period: config.epoch_period,
        threshold_deposit_rate: config.threshold_deposit_rate,
        target_deposit_rate: config.target_deposit_rate,
        buffer_distribution_factor: config.buffer_distribution_factor,
        anc_purchase_factor: config.anc_purchase_factor,
        price_timeframe: config.price_timeframe,
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
                name: whitelist_elem.name,
                symbol: whitelist_elem.symbol,
                max_ltv: whitelist_elem.max_ltv,
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
        threshold_deposit_rate: config.threshold_deposit_rate,
    })
}
