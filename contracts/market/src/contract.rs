use crate::borrow::{
    borrow_stable, claim_rewards, compute_interest, compute_interest_raw, compute_reward,
    query_liabilities, query_liability, query_loan_amount, repay_stable,
    repay_stable_from_liquidation,
};
use crate::deposit::{compute_exchange_rate_raw, deposit_stable, redeem_stable};
use crate::querier::{query_anc_emission_rate, query_borrow_rate};
use crate::state::{read_config, read_state, store_config, store_state, Config, State};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    from_binary, log, to_binary, Api, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Env, Extern,
    HandleResponse, HandleResult, HumanAddr, InitResponse, InitResult, Querier, StdError,
    StdResult, Storage, Uint128, WasmMsg,
};
use cw20::{Cw20CoinHuman, Cw20ReceiveMsg, MinterResponse};

use moneymarket::interest::BorrowRateResponse;
use moneymarket::market::{
    ConfigResponse, Cw20HookMsg, EpochStateResponse, HandleMsg, InitMsg, QueryMsg, StateResponse,
};
use moneymarket::querier::{deduct_tax, query_balance, query_supply};
use terraswap::hook::InitHook;
use terraswap::token::InitMsg as TokenInitMsg;

pub const INITIAL_DEPOSIT_AMOUNT: u128 = 1000000;
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let initial_deposit = env
        .message
        .sent_funds
        .iter()
        .find(|c| c.denom == msg.stable_denom)
        .map(|c| c.amount)
        .unwrap_or_else(|| Uint128::zero());

    if initial_deposit != Uint128(INITIAL_DEPOSIT_AMOUNT) {
        return Err(StdError::generic_err(format!(
            "Must deposit initial funds {:?}{:?}",
            INITIAL_DEPOSIT_AMOUNT,
            msg.stable_denom.clone()
        )));
    }

    store_config(
        &mut deps.storage,
        &Config {
            contract_addr: deps.api.canonical_address(&env.contract.address)?,
            owner_addr: deps.api.canonical_address(&msg.owner_addr)?,
            atoken_contract: CanonicalAddr::default(),
            overseer_contract: CanonicalAddr::default(),
            interest_model: deps.api.canonical_address(&msg.interest_model)?,
            distribution_model: deps.api.canonical_address(&msg.distribution_model)?,
            collector_contract: deps.api.canonical_address(&msg.collector_contract)?,
            faucet_contract: deps.api.canonical_address(&msg.faucet_contract)?,
            stable_denom: msg.stable_denom.clone(),
            reserve_factor: msg.reserve_factor,
            max_borrow_factor: msg.max_borrow_factor,
        },
    )?;

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: msg.anc_emission_rate,
        },
    )?;

    Ok(InitResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: msg.atoken_code_id,
            send: vec![],
            label: None,
            msg: to_binary(&TokenInitMsg {
                name: format!("Anchor Token for {}", msg.stable_denom),
                symbol: format!("AT-{}", msg.stable_denom),
                decimals: 6u8,
                initial_balances: vec![Cw20CoinHuman {
                    address: env.contract.address.clone(),
                    amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
                }],
                mint: Some(MinterResponse {
                    minter: env.contract.address.clone(),
                    cap: None,
                }),
                init_hook: Some(InitHook {
                    contract_addr: env.contract.address,
                    msg: to_binary(&HandleMsg::RegisterAToken {})?,
                }),
            })?,
        })],
        log: vec![],
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::RegisterAToken {} => register_atoken(deps, env),
        HandleMsg::RegisterOverseer { overseer_contract } => {
            register_overseer_contract(deps, overseer_contract)
        }
        HandleMsg::UpdateConfig {
            owner_addr,
            reserve_factor,
            interest_model,
            distribution_model,
        } => update_config(
            deps,
            env,
            owner_addr,
            reserve_factor,
            interest_model,
            distribution_model,
        ),
        HandleMsg::ExecuteEpochOperations {
            target_deposit_rate,
            deposit_rate,
        } => execute_epoch_operations(deps, env, target_deposit_rate, deposit_rate),
        HandleMsg::DepositStable {} => deposit_stable(deps, env),
        HandleMsg::BorrowStable { borrow_amount, to } => {
            borrow_stable(deps, env, borrow_amount, to)
        }
        HandleMsg::RepayStable {} => repay_stable(deps, env),
        HandleMsg::RepayStableFromLiquidation {
            borrower,
            prev_balance,
        } => repay_stable_from_liquidation(deps, env, borrower, prev_balance),
        HandleMsg::ClaimRewards {} => claim_rewards(deps, env),
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    let contract_addr = env.message.sender.clone();
    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::RedeemStable {} => {
                // only asset contract can execute this message
                let config: Config = read_config(&deps.storage)?;
                if deps.api.canonical_address(&contract_addr)? != config.atoken_contract {
                    return Err(StdError::unauthorized());
                }

                redeem_stable(deps, env, cw20_msg.sender, cw20_msg.amount)
            }
        }
    } else {
        Err(StdError::generic_err(
            "Invalid request: \"redeem stable\" message not included in request",
        ))
    }
}

pub fn register_atoken<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if config.atoken_contract != CanonicalAddr::default() {
        return Err(StdError::unauthorized());
    }

    config.atoken_contract = deps.api.canonical_address(&env.message.sender)?;
    store_config(&mut deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("atoken", env.message.sender)],
        data: None,
    })
}

pub fn register_overseer_contract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    overseer_contract: HumanAddr,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if config.overseer_contract != CanonicalAddr::default() {
        return Err(StdError::unauthorized());
    }

    config.overseer_contract = deps.api.canonical_address(&overseer_contract)?;
    store_config(&mut deps.storage, &config)?;

    Ok(HandleResponse::default())
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner_addr: Option<HumanAddr>,
    reserve_factor: Option<Decimal256>,
    interest_model: Option<HumanAddr>,
    distribution_model: Option<HumanAddr>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;

    // permission check
    if deps.api.canonical_address(&env.message.sender)? != config.owner_addr {
        return Err(StdError::unauthorized());
    }

    if let Some(owner_addr) = owner_addr {
        config.owner_addr = deps.api.canonical_address(&owner_addr)?;
    }

    if reserve_factor.is_some() || interest_model.is_some() {
        let mut state: State = read_state(&deps.storage)?;
        compute_interest(&deps, &config, &mut state, env.block.height, None)?;
        store_state(&mut deps.storage, &state)?;

        if let Some(interest_model) = interest_model {
            config.interest_model = deps.api.canonical_address(&interest_model)?;
        }

        if let Some(reserve_factor) = reserve_factor {
            config.reserve_factor = reserve_factor;
        }
    }

    if let Some(distribution_model) = distribution_model {
        config.distribution_model = deps.api.canonical_address(&distribution_model)?;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}

pub fn execute_epoch_operations<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    target_deposit_rate: Decimal256,
    deposit_rate: Decimal256,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let mut state: State = read_state(&deps.storage)?;

    // Compute distributed rewards before updating anc_emission_rate
    compute_interest(&deps, &config, &mut state, env.block.height, None)?;
    compute_reward(&mut state, env.block.height);

    // Query updated anc_emission_rate
    let anc_emission_rate_res = query_anc_emission_rate(
        &deps,
        &deps.api.human_address(&config.distribution_model)?,
        target_deposit_rate,
        deposit_rate,
        state.anc_emission_rate,
    )?;

    // Compute total_reserves to fund collector contract
    let total_reserves = state.total_reserves * Uint256::one();
    let messages: Vec<CosmosMsg> = if !total_reserves.is_zero() {
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: deps.api.human_address(&config.collector_contract)?,
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: config.stable_denom,
                    amount: total_reserves.into(),
                },
            )?],
        })]
    } else {
        vec![]
    };

    state.anc_emission_rate = anc_emission_rate_res.emission_rate;
    state.total_reserves = state.total_reserves - Decimal256::from_uint256(total_reserves);
    store_state(&mut deps.storage, &state)?;

    return Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "execute_epoch_operations"),
            log("total_reserves", total_reserves),
            log("anc_emission_rate", state.anc_emission_rate),
        ],
        data: None,
    });
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::EpochState { block_height } => to_binary(&query_epoch_state(deps, block_height)?),
        QueryMsg::Liability { borrower } => to_binary(&query_liability(deps, borrower)?),
        QueryMsg::Liabilities { start_after, limit } => {
            to_binary(&query_liabilities(deps, start_after, limit)?)
        }
        QueryMsg::LoanAmount {
            borrower,
            block_height,
        } => to_binary(&query_loan_amount(deps, borrower, block_height)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config: Config = read_config(&deps.storage)?;
    Ok(ConfigResponse {
        owner_addr: deps.api.human_address(&config.owner_addr)?,
        atoken_contract: deps.api.human_address(&config.atoken_contract)?,
        interest_model: deps.api.human_address(&config.interest_model)?,
        overseer_contract: deps.api.human_address(&config.overseer_contract)?,
        collector_contract: deps.api.human_address(&config.collector_contract)?,
        faucet_contract: deps.api.human_address(&config.faucet_contract)?,
        stable_denom: config.stable_denom,
        reserve_factor: config.reserve_factor,
    })
}

pub fn query_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<StateResponse> {
    let state: State = read_state(&deps.storage)?;
    Ok(StateResponse {
        total_liabilities: state.total_liabilities,
        total_reserves: state.total_reserves,
        last_interest_updated: state.last_interest_updated,
        global_interest_index: state.global_interest_index,
        global_reward_index: state.global_reward_index,
        anc_emission_rate: state.anc_emission_rate,
    })
}

pub fn query_epoch_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_height: Option<u64>,
) -> StdResult<EpochStateResponse> {
    let config: Config = read_config(&deps.storage)?;
    let mut state: State = read_state(&deps.storage)?;

    let a_token_supply = query_supply(&deps, &deps.api.human_address(&config.atoken_contract)?)?;
    let balance = query_balance(
        &deps,
        &deps.api.human_address(&config.contract_addr)?,
        config.stable_denom.to_string(),
    )?;

    if let Some(block_height) = block_height {
        let borrow_rate_res: BorrowRateResponse = query_borrow_rate(
            &deps,
            &deps.api.human_address(&config.interest_model)?,
            balance,
            state.total_liabilities,
            state.total_reserves,
        )?;

        // Compute interest rate to return latest epoch state
        compute_interest_raw(
            &mut state,
            block_height,
            borrow_rate_res.rate,
            config.reserve_factor,
        );
    }

    let exchange_rate = compute_exchange_rate_raw(&state, a_token_supply, balance);

    Ok(EpochStateResponse {
        exchange_rate,
        a_token_supply,
    })
}
