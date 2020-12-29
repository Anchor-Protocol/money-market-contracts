use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::borrow::compute_interest;
use crate::state::{read_config, read_state, store_state, Config, State};

use cw20::Cw20HandleMsg;
use moneymarket::{deduct_tax, query_balance, query_supply};

pub fn deposit_stable<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;

    // Check base denom deposit
    let deposit_amount: Uint256 = env
        .message
        .sent_funds
        .iter()
        .find(|c| c.denom == config.stable_denom)
        .map(|c| Uint256::from(c.amount))
        .unwrap_or_else(Uint256::zero);

    // Cannot deposit zero amount
    if deposit_amount.is_zero() {
        return Err(StdError::generic_err("Cannot deposit zero coins"));
    }

    // Update interest related state
    let mut state: State = read_state(&deps.storage)?;
    compute_interest(
        &deps,
        &config,
        &mut state,
        env.block.height,
        Some(deposit_amount),
    )?;
    store_state(&mut deps.storage, &state)?;

    // Load anchor token exchange rate with updated state
    let exchange_rate = compute_exchange_rate(deps, &config, &state, Some(deposit_amount))?;
    let mint_amount = deposit_amount / exchange_rate;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.anchor_token)?,
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Mint {
                recipient: env.message.sender.clone(),
                amount: mint_amount.into(),
            })?,
        })],
        log: vec![
            log("action", "deposit_stable"),
            log("depositor", env.message.sender),
            log("mint_amount", mint_amount),
            log("deposit_amount", deposit_amount),
        ],
        data: None,
    })
}

pub fn redeem_stable<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    burn_amount: Uint128,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;

    // Update interest related state
    let mut state: State = read_state(&deps.storage)?;
    compute_interest(&deps, &config, &mut state, env.block.height, None)?;
    // Load anchor token exchange rate with updated state
    let exchange_rate = compute_exchange_rate(deps, &config, &state, None)?;
    let redeem_amount = Uint256::from(burn_amount) * exchange_rate;
    if redeem_amount
        > query_balance(
            &deps,
            &env.contract.address,
            config.stable_denom.to_string(),
        )?
    {
        return Err(StdError::generic_err(
            "Failed to redeem stable; not enough contract balance",
        ));
    }

    store_state(&mut deps.storage, &state)?;
    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.human_address(&config.anchor_token)?,
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Burn {
                    amount: burn_amount,
                })?,
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address,
                to_address: sender,
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: config.stable_denom,
                        amount: redeem_amount.into(),
                    },
                )?],
            }),
        ],
        log: vec![
            log("action", "redeem_stable"),
            log("burn_amount", burn_amount),
            log("redeem_amount", redeem_amount),
        ],
        data: None,
    })
}

pub(crate) fn compute_exchange_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    config: &Config,
    state: &State,
    deposit_amount: Option<Uint256>,
) -> StdResult<Decimal256> {
    let a_token_supply = query_supply(&deps, &deps.api.human_address(&config.anchor_token)?)?;
    if a_token_supply.is_zero() {
        return Ok(Decimal256::one());
    }

    let balance = query_balance(
        &deps,
        &deps.api.human_address(&config.contract_addr)?,
        config.stable_denom.to_string(),
    )? - deposit_amount.unwrap_or_else(Uint256::zero);

    compute_exchange_rate_raw(state, a_token_supply, balance)
}

pub fn compute_exchange_rate_raw(
    state: &State,
    a_token_supply: Uint256,
    contract_balance: Uint256,
) -> StdResult<Decimal256> {
    // (anchor_token / stable_denom)
    // exchange_rate = (balance + total_liabilities - total_reservess) / anchor_token_supply
    Ok(
        (Decimal256::from_uint256(contract_balance) + state.total_liabilities
            - state.total_reservess)
            / Decimal256::from_uint256(a_token_supply),
    )
}
