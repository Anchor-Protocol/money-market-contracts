#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg,
};
use std::cmp::{max, min};

use crate::collateral::{
    liquidate_collateral, lock_collateral, query_all_collaterals, query_borrow_limit,
    query_collaterals, unlock_collateral,
};
use crate::error::ContractError;
use crate::querier::query_epoch_state;

use crate::state::{
    read_config, read_dynrate_config, read_dynrate_state, read_epoch_state, read_whitelist,
    read_whitelist_elem, store_config, store_dynrate_config, store_dynrate_state,
    store_epoch_state, store_whitelist_elem, Config, DynrateConfig, DynrateState, EpochState,
    WhitelistElem,
};

use cosmwasm_bignumber::{Decimal256, Uint256};
use moneymarket::common::optional_addr_validate;
use moneymarket::custody::ExecuteMsg as CustodyExecuteMsg;
use moneymarket::market::EpochStateResponse;
use moneymarket::market::ExecuteMsg as MarketExecuteMsg;
use moneymarket::overseer::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, WhitelistResponse,
    WhitelistResponseElem,
};
use moneymarket::querier::{deduct_tax, query_balance};

pub const BLOCKS_PER_YEAR: u128 = 4656810;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner_addr: deps.api.addr_canonicalize(&msg.owner_addr)?,
            oracle_contract: deps.api.addr_canonicalize(&msg.oracle_contract)?,
            market_contract: deps.api.addr_canonicalize(&msg.market_contract)?,
            liquidation_contract: deps.api.addr_canonicalize(&msg.liquidation_contract)?,
            collector_contract: deps.api.addr_canonicalize(&msg.collector_contract)?,
            stable_denom: msg.stable_denom,
            epoch_period: msg.epoch_period,
            threshold_deposit_rate: msg.threshold_deposit_rate,
            target_deposit_rate: msg.target_deposit_rate,
            buffer_distribution_factor: msg.buffer_distribution_factor,
            anc_purchase_factor: msg.anc_purchase_factor,
            price_timeframe: msg.price_timeframe,
        },
    )?;

    store_dynrate_config(
        deps.storage,
        &DynrateConfig {
            dyn_rate_epoch: msg.dyn_rate_epoch,
            dyn_rate_maxchange: msg.dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation: msg.dyn_rate_yr_increase_expectation,
            dyn_rate_min: msg.dyn_rate_min,
            dyn_rate_max: msg.dyn_rate_max,
        },
    )?;

    store_epoch_state(
        deps.storage,
        &EpochState {
            deposit_rate: Decimal256::zero(),
            prev_aterra_supply: Uint256::zero(),
            prev_interest_buffer: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
            last_executed_height: env.block.height,
        },
    )?;

    store_dynrate_state(
        deps.storage,
        &DynrateState {
            last_executed_height: env.block.height,
            prev_yield_reserve: Decimal256::zero(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    store_dynrate_config(
        deps.storage,
        &DynrateConfig {
            dyn_rate_epoch: msg.dyn_rate_epoch,
            dyn_rate_maxchange: msg.dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation: msg.dyn_rate_yr_increase_expectation,
            dyn_rate_min: msg.dyn_rate_min,
            dyn_rate_max: msg.dyn_rate_max,
        },
    )?;
    let mut config = read_config(deps.storage)?;
    let prev_yield_reserve = query_balance(
        deps.as_ref(),
        env.contract.address.clone(),
        config.stable_denom.clone(),
    )?;
    store_dynrate_state(
        deps.storage,
        &DynrateState {
            last_executed_height: env.block.height,
            prev_yield_reserve: Decimal256::from_ratio(prev_yield_reserve, 1),
        },
    )?;
    let new_rate = max(
        min(config.threshold_deposit_rate, msg.dyn_rate_max),
        msg.dyn_rate_min,
    );
    config.threshold_deposit_rate = new_rate;
    config.target_deposit_rate = new_rate;
    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner_addr,
            oracle_contract,
            liquidation_contract,
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
            anc_purchase_factor,
            epoch_period,
            price_timeframe,
            dyn_rate_epoch,
            dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation,
            dyn_rate_min,
            dyn_rate_max,
        } => {
            let api = deps.api;
            update_config(
                deps,
                info,
                optional_addr_validate(api, owner_addr)?,
                optional_addr_validate(api, oracle_contract)?,
                optional_addr_validate(api, liquidation_contract)?,
                threshold_deposit_rate,
                target_deposit_rate,
                buffer_distribution_factor,
                anc_purchase_factor,
                epoch_period,
                price_timeframe,
                dyn_rate_epoch,
                dyn_rate_maxchange,
                dyn_rate_yr_increase_expectation,
                dyn_rate_min,
                dyn_rate_max,
            )
        }
        ExecuteMsg::Whitelist {
            name,
            symbol,
            collateral_token,
            custody_contract,
            max_ltv,
        } => {
            let api = deps.api;
            register_whitelist(
                deps,
                info,
                name,
                symbol,
                api.addr_validate(&collateral_token)?,
                api.addr_validate(&custody_contract)?,
                max_ltv,
            )
        }
        ExecuteMsg::UpdateWhitelist {
            collateral_token,
            custody_contract,
            max_ltv,
        } => {
            let api = deps.api;
            update_whitelist(
                deps,
                info,
                api.addr_validate(&collateral_token)?,
                optional_addr_validate(api, custody_contract)?,
                max_ltv,
            )
        }
        ExecuteMsg::ExecuteEpochOperations {} => execute_epoch_operations(deps, env),
        ExecuteMsg::UpdateEpochState {
            interest_buffer,
            distributed_interest,
        } => update_epoch_state(deps, env, info, interest_buffer, distributed_interest),
        ExecuteMsg::LockCollateral { collaterals } => lock_collateral(deps, info, collaterals),
        ExecuteMsg::UnlockCollateral { collaterals } => {
            unlock_collateral(deps, env, info, collaterals)
        }
        ExecuteMsg::LiquidateCollateral { borrower } => {
            let api = deps.api;
            liquidate_collateral(deps, env, info, api.addr_validate(&borrower)?)
        }
        ExecuteMsg::FundReserve {} => fund_reserve(deps, info),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner_addr: Option<Addr>,
    oracle_contract: Option<Addr>,
    liquidation_contract: Option<Addr>,
    threshold_deposit_rate: Option<Decimal256>,
    target_deposit_rate: Option<Decimal256>,
    buffer_distribution_factor: Option<Decimal256>,
    anc_purchase_factor: Option<Decimal256>,
    epoch_period: Option<u64>,
    price_timeframe: Option<u64>,
    dyn_rate_epoch: Option<u64>,
    dyn_rate_maxchange: Option<Decimal256>,
    dyn_rate_yr_increase_expectation: Option<Decimal256>,
    dyn_rate_min: Option<Decimal256>,
    dyn_rate_max: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    let mut dynrate_config: DynrateConfig = read_dynrate_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner_addr) = owner_addr {
        config.owner_addr = deps.api.addr_canonicalize(&owner_addr.to_string())?;
    }

    if let Some(oracle_contract) = oracle_contract {
        config.oracle_contract = deps.api.addr_canonicalize(&oracle_contract.to_string())?;
    }

    if let Some(liquidation_contract) = liquidation_contract {
        config.liquidation_contract = deps
            .api
            .addr_canonicalize(&liquidation_contract.to_string())?;
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

    if let Some(dyn_rate_epoch) = dyn_rate_epoch {
        dynrate_config.dyn_rate_epoch = dyn_rate_epoch;
    }

    if let Some(dyn_rate_maxchange) = dyn_rate_maxchange {
        dynrate_config.dyn_rate_maxchange = dyn_rate_maxchange;
    }

    if let Some(dyn_rate_yr_increase_expectation) = dyn_rate_yr_increase_expectation {
        dynrate_config.dyn_rate_yr_increase_expectation = dyn_rate_yr_increase_expectation;
    }

    if let Some(dyn_rate_min) = dyn_rate_min {
        dynrate_config.dyn_rate_min = dyn_rate_min;
    }

    if let Some(dyn_rate_max) = dyn_rate_max {
        dynrate_config.dyn_rate_max = dyn_rate_max;
    }

    store_config(deps.storage, &config)?;
    store_dynrate_config(deps.storage, &dynrate_config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn register_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    symbol: String,
    collateral_token: Addr,
    custody_contract: Addr,
    max_ltv: Decimal256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    if read_whitelist_elem(deps.storage, &collateral_token_raw).is_ok() {
        return Err(ContractError::TokenAlreadyRegistered {});
    }

    store_whitelist_elem(
        deps.storage,
        &collateral_token_raw,
        &WhitelistElem {
            name: name.to_string(),
            symbol: symbol.to_string(),
            custody_contract: deps.api.addr_canonicalize(custody_contract.as_str())?,
            max_ltv,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_whitelist"),
        attr("name", name),
        attr("symbol", symbol),
        attr("collateral_token", collateral_token),
        attr("custody_contract", custody_contract),
        attr("LTV", max_ltv.to_string()),
    ]))
}

pub fn update_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    collateral_token: Addr,
    custody_contract: Option<Addr>,
    max_ltv: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    let mut whitelist_elem: WhitelistElem =
        read_whitelist_elem(deps.storage, &collateral_token_raw)?;

    if let Some(custody_contract) = custody_contract {
        whitelist_elem.custody_contract = deps.api.addr_canonicalize(custody_contract.as_str())?;
    }

    if let Some(max_ltv) = max_ltv {
        whitelist_elem.max_ltv = max_ltv;
    }

    store_whitelist_elem(deps.storage, &collateral_token_raw, &whitelist_elem)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_whitelist"),
        attr("collateral_token", collateral_token),
        attr(
            "custody_contract",
            deps.api.addr_humanize(&whitelist_elem.custody_contract)?,
        ),
        attr("LTV", whitelist_elem.max_ltv.to_string()),
    ]))
}

fn update_deposit_rate(deps: DepsMut, env: Env) -> StdResult<()> {
    let dynrate_config: DynrateConfig = read_dynrate_config(deps.storage)?;
    let dynrate_state: DynrateState = read_dynrate_state(deps.storage)?;
    let mut config: Config = read_config(deps.storage)?;

    // check whether its time to re-evaluate rate
    if env.block.height >= dynrate_state.last_executed_height + dynrate_config.dyn_rate_epoch {
        // retrieve interest buffer
        let interest_buffer = query_balance(
            deps.as_ref(),
            env.contract.address.clone(),
            config.stable_denom.to_string(),
        )?;
        // convert block rate into yearly rate
        let blocks_per_year = Decimal256::from_ratio(Uint256::from(BLOCKS_PER_YEAR), 1);
        let current_rate = config.threshold_deposit_rate * blocks_per_year;

        let yield_reserve = Decimal256::from_uint256(interest_buffer);
        let mut yr_went_up = yield_reserve > dynrate_state.prev_yield_reserve;

        // amount yield reserve changed in notional terms
        let yield_reserve_delta = if yr_went_up {
            yield_reserve - dynrate_state.prev_yield_reserve
        } else {
            dynrate_state.prev_yield_reserve - yield_reserve
        };

        // amount yield reserve changed in percentage terms
        // if the prev yield reserve was zero; assume either a 100% decrease
        // or a 100% increase, but this should be very rare
        let mut yield_reserve_change = if dynrate_state.prev_yield_reserve.is_zero() {
            Decimal256::one()
        } else {
            yield_reserve_delta / dynrate_state.prev_yield_reserve
        };

        // decreases the yield reserve change by dyn_rate_yr_increase_expectation
        // (assume (yr_went_up, yield_reserve_change) is one signed integer, this just subtracts
        // that integer by dynrate_config.dyn_rate_yr_increase_expectation)
        let increase_expectation = dynrate_config.dyn_rate_yr_increase_expectation;
        yield_reserve_change = if !yr_went_up {
            yield_reserve_change + increase_expectation
        } else if yield_reserve_change > increase_expectation {
            yield_reserve_change - increase_expectation
        } else {
            yr_went_up = !yr_went_up;
            increase_expectation - yield_reserve_change
        };

        yield_reserve_change = min(yield_reserve_change, dynrate_config.dyn_rate_maxchange);

        let mut new_rate = if yr_went_up {
            current_rate + yield_reserve_change
        } else if current_rate > yield_reserve_change {
            current_rate - yield_reserve_change
        } else {
            Decimal256::zero()
        };

        // convert from yearly rate to block rate
        new_rate = new_rate / blocks_per_year;

        // clamp new rate
        new_rate = max(
            min(new_rate, dynrate_config.dyn_rate_max),
            dynrate_config.dyn_rate_min,
        );

        config.target_deposit_rate = new_rate;
        config.threshold_deposit_rate = new_rate;
        store_config(deps.storage, &config)?;

        // store updated epoch state
        store_dynrate_state(
            deps.storage,
            &DynrateState {
                last_executed_height: env.block.height,
                prev_yield_reserve: yield_reserve,
            },
        )?;
    };
    Ok(())
}

pub fn execute_epoch_operations(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let state: EpochState = read_epoch_state(deps.storage)?;
    if env.block.height < state.last_executed_height + config.epoch_period {
        return Err(ContractError::EpochNotPassed(state.last_executed_height));
    }

    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - state.last_executed_height);

    // Compute next epoch state
    let market_contract = deps.api.addr_humanize(&config.market_contract)?;
    let epoch_state: EpochStateResponse = query_epoch_state(
        deps.as_ref(),
        market_contract.clone(),
        env.block.height,
        None,
    )?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate = epoch_state.exchange_rate / state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut interest_buffer = query_balance(
        deps.as_ref(),
        env.contract.address.clone(),
        config.stable_denom.to_string(),
    )?;

    // Send accrued_buffer * config.anc_purchase_factor amount stable token to collector
    let accrued_buffer = interest_buffer - state.prev_interest_buffer;
    let anc_purchase_amount = accrued_buffer * config.anc_purchase_factor;
    if !anc_purchase_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: deps
                .api
                .addr_humanize(&config.collector_contract)?
                .to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
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
            // deduct tax
            distributed_interest = Uint256::from(
                deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: config.stable_denom.to_string(),
                        amount: distributed_interest.into(),
                    },
                )?
                .amount,
            );

            // Send some portion of interest buffer to Market contract
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: market_contract.to_string(),
                amount: vec![Coin {
                    denom: config.stable_denom,
                    amount: distributed_interest.into(),
                }],
            }));
        }
    }

    // Execute DistributeRewards
    let whitelist: Vec<WhitelistResponseElem> = read_whitelist(deps.as_ref(), None, None)?;
    for elem in whitelist.iter() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: elem.custody_contract.clone(),
            funds: vec![],
            msg: to_binary(&CustodyExecuteMsg::DistributeRewards {})?,
        }));
    }

    // TODO: Should this become a reply? If so which SubMsg to make reply_on?
    // Execute store epoch state operation
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::UpdateEpochState {
            interest_buffer,
            distributed_interest,
        })?,
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "epoch_operations"),
        attr("deposit_rate", deposit_rate.to_string()),
        attr("exchange_rate", epoch_state.exchange_rate.to_string()),
        attr("aterra_supply", epoch_state.aterra_supply),
        attr("distributed_interest", distributed_interest),
        attr("anc_purchase_amount", anc_purchase_amount),
    ]))
}

pub fn update_epoch_state(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // To store interest buffer before receiving epoch staking rewards,
    // pass interest_buffer from execute_epoch_operations
    interest_buffer: Uint256,
    distributed_interest: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let overseer_epoch_state: EpochState = read_epoch_state(deps.storage)?;
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - overseer_epoch_state.last_executed_height);

    // Compute next epoch state
    let market_contract = deps.api.addr_humanize(&config.market_contract)?;
    let market_epoch_state: EpochStateResponse = query_epoch_state(
        deps.as_ref(),
        market_contract.clone(),
        env.block.height,
        Some(distributed_interest),
    )?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate =
        market_epoch_state.exchange_rate / overseer_epoch_state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    // store updated epoch state
    store_epoch_state(
        deps.storage,
        &EpochState {
            last_executed_height: env.block.height,
            prev_aterra_supply: market_epoch_state.aterra_supply,
            prev_exchange_rate: market_epoch_state.exchange_rate,
            prev_interest_buffer: interest_buffer,
            deposit_rate,
        },
    )?;

    // use unchanged rates to build msg
    let response_msg = to_binary(&MarketExecuteMsg::ExecuteEpochOperations {
        deposit_rate,
        target_deposit_rate: config.target_deposit_rate,
        threshold_deposit_rate: config.threshold_deposit_rate,
        distributed_interest,
    })?;

    // proceed with deposit rate update
    update_deposit_rate(deps, env)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market_contract.to_string(),
            funds: vec![],
            msg: response_msg,
        }))
        .add_attributes(vec![
            attr("action", "update_epoch_state"),
            attr("deposit_rate", deposit_rate.to_string()),
            attr("aterra_supply", market_epoch_state.aterra_supply),
            attr(
                "exchange_rate",
                market_epoch_state.exchange_rate.to_string(),
            ),
            attr("interest_buffer", interest_buffer),
        ]))
}

pub fn fund_reserve(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let sent_uusd = match info.funds.iter().find(|x| x.denom == "uusd") {
        Some(coin) => coin.amount,
        None => Uint128::zero(),
    };

    let mut overseer_epoch_state: EpochState = read_epoch_state(deps.storage)?;
    overseer_epoch_state.prev_interest_buffer += Uint256::from(sent_uusd);
    store_epoch_state(deps.storage, &overseer_epoch_state)?;

    let mut dyn_rate_state: DynrateState = read_dynrate_state(deps.storage)?;
    dyn_rate_state.prev_yield_reserve += Decimal256::from_ratio(Uint256::from(sent_uusd), 1);
    store_dynrate_state(deps.storage, &dyn_rate_state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "fund_reserve"),
        attr("funded_amount", sent_uusd.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::EpochState {} => to_binary(&query_state(deps)?),
        QueryMsg::Whitelist {
            collateral_token,
            start_after,
            limit,
        } => to_binary(&query_whitelist(
            deps,
            optional_addr_validate(deps.api, collateral_token)?,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
        QueryMsg::Collaterals { borrower } => to_binary(&query_collaterals(
            deps,
            deps.api.addr_validate(&borrower)?,
        )?),
        QueryMsg::AllCollaterals { start_after, limit } => to_binary(&query_all_collaterals(
            deps,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
        QueryMsg::BorrowLimit {
            borrower,
            block_time,
        } => to_binary(&query_borrow_limit(
            deps,
            deps.api.addr_validate(&borrower)?,
            block_time,
        )?),
        QueryMsg::DynrateState {} => to_binary(&query_dynrate_state(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    let dynrate_config: DynrateConfig = read_dynrate_config(deps.storage)?;
    Ok(ConfigResponse {
        owner_addr: deps.api.addr_humanize(&config.owner_addr)?.to_string(),
        oracle_contract: deps.api.addr_humanize(&config.oracle_contract)?.to_string(),
        market_contract: deps.api.addr_humanize(&config.market_contract)?.to_string(),
        liquidation_contract: deps
            .api
            .addr_humanize(&config.liquidation_contract)?
            .to_string(),
        collector_contract: deps
            .api
            .addr_humanize(&config.collector_contract)?
            .to_string(),
        stable_denom: config.stable_denom,
        epoch_period: config.epoch_period,
        threshold_deposit_rate: config.threshold_deposit_rate,
        target_deposit_rate: config.target_deposit_rate,
        buffer_distribution_factor: config.buffer_distribution_factor,
        anc_purchase_factor: config.anc_purchase_factor,
        price_timeframe: config.price_timeframe,
        dyn_rate_epoch: dynrate_config.dyn_rate_epoch,
        dyn_rate_maxchange: dynrate_config.dyn_rate_maxchange,
        dyn_rate_yr_increase_expectation: dynrate_config.dyn_rate_yr_increase_expectation,
        dyn_rate_min: dynrate_config.dyn_rate_min,
        dyn_rate_max: dynrate_config.dyn_rate_max,
    })
}

pub fn query_state(deps: Deps) -> StdResult<EpochState> {
    read_epoch_state(deps.storage)
}

pub fn query_dynrate_state(deps: Deps) -> StdResult<DynrateState> {
    read_dynrate_state(deps.storage)
}

pub fn query_whitelist(
    deps: Deps,
    collateral_token: Option<Addr>,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<WhitelistResponse> {
    if let Some(collateral_token) = collateral_token {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(
            deps.storage,
            &deps.api.addr_canonicalize(collateral_token.as_str())?,
        )?;
        Ok(WhitelistResponse {
            elems: vec![WhitelistResponseElem {
                name: whitelist_elem.name,
                symbol: whitelist_elem.symbol,
                max_ltv: whitelist_elem.max_ltv,
                custody_contract: deps
                    .api
                    .addr_humanize(&whitelist_elem.custody_contract)?
                    .to_string(),
                collateral_token: collateral_token.to_string(),
            }],
        })
    } else {
        let start_after = if let Some(start_after) = start_after {
            Some(deps.api.addr_canonicalize(start_after.as_str())?)
        } else {
            None
        };

        let whitelist: Vec<WhitelistResponseElem> = read_whitelist(deps, start_after, limit)?;
        Ok(WhitelistResponse { elems: whitelist })
    }
}
