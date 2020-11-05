use cosmwasm_std::{
    from_binary, log, to_binary, Api, CanonicalAddr, CosmosMsg, Decimal, Env, Extern,
    HandleResponse, HandleResult, HumanAddr, InitResponse, InitResult, Querier, StdError, Storage,
    Uint128, WasmMsg,
};

use crate::borrow::{borrow_stable, repay_stable};
use crate::deposit::{deposit_stable, redeem_stable};
use crate::msg::{Cw20HookMsg, HandleMsg, InitMsg};
use crate::state::{read_config, store_config, store_state, Config, State};

use cw20::{Cw20ReceiveMsg, MinterResponse};
use terraswap::{InitHook, TokenInitMsg};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    store_config(
        &mut deps.storage,
        &Config {
            owner_addr: deps.api.canonical_address(&msg.owner_addr)?,
            anchor_token: CanonicalAddr::default(),
            overseer_contract: CanonicalAddr::default(),
            interest_model: deps.api.canonical_address(&msg.interest_model)?,
            base_denom: msg.base_denom.clone(),
            reserve_factor: msg.reserve_factor,
        },
    )?;

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Uint128::zero(),
            total_reserves: Uint128::zero(),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal::zero(),
        },
    )?;

    Ok(InitResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: msg.anchor_token_code_id,
            send: vec![],
            label: None,
            msg: to_binary(&TokenInitMsg {
                name: format!("Anchor Token for {}", msg.base_denom),
                symbol: format!("AT-{}", msg.base_denom),
                decimals: 6u8,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.clone(),
                    cap: None,
                }),
                init_hook: Some(InitHook {
                    contract_addr: env.contract.address,
                    msg: to_binary(&HandleMsg::RegisterAnchorToken {})?,
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
        HandleMsg::RegisterAnchorToken {} => register_anchor_token(deps, env),
        HandleMsg::RegisterOverseer { overseer_contract } => {
            register_overseer_contract(deps, overseer_contract)
        }
        HandleMsg::UpdateConfig {
            owner_addr,
            interest_model,
            reserve_factor,
        } => update_config(deps, env, owner_addr, interest_model, reserve_factor),
        HandleMsg::DepositStable {} => deposit_stable(deps, env),
        HandleMsg::BorrowStable { borrow_amount, to } => {
            borrow_stable(deps, env, borrow_amount, to)
        }
        HandleMsg::RepayStable {} => repay_stable(deps, env),
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
                if deps.api.canonical_address(&contract_addr)? != config.anchor_token {
                    return Err(StdError::unauthorized());
                }

                redeem_stable(deps, env, cw20_msg.sender, cw20_msg.amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

pub fn register_anchor_token<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if config.anchor_token != CanonicalAddr::default() {
        return Err(StdError::unauthorized());
    }

    config.anchor_token = deps.api.canonical_address(&env.message.sender)?;
    store_config(&mut deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("anchor_token", env.message.sender)],
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
    interest_model: Option<HumanAddr>,
    reserve_factor: Option<Decimal>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;

    // permission check
    if deps.api.canonical_address(&env.message.sender)? != config.owner_addr {
        return Err(StdError::unauthorized());
    }

    if let Some(owner_addr) = owner_addr {
        config.owner_addr = deps.api.canonical_address(&owner_addr)?;
    }

    if let Some(interest_model) = interest_model {
        config.interest_model = deps.api.canonical_address(&interest_model)?;
    }

    if let Some(reserve_factor) = reserve_factor {
        config.reserve_factor = reserve_factor;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}
