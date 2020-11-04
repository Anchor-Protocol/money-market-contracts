use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::msg::{AllCollateralsResponse, BorrowLimitResponse, CollateralsResponse};
use crate::state::{
    read_all_collaterals, read_collaterals, read_config, read_whitelist_elem, store_collaterals,
    Config, Tokens, TokensMath, WhitelistElem,
};
use moneymarket::{
    load_borrow_amount, load_oracle_price, BorrowAmountResponse, CustodyHandleMsg,
    OraclePriceResponse,
};

pub fn handle_lock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collaterals: Vec<(HumanAddr, Uint128)>,
) -> HandleResult {
    let borrower_raw = deps.api.canonical_address(&env.message.sender)?;
    let mut cur_collaterals: Tokens = read_collaterals(&deps.storage, &borrower_raw);

    let collaterals_raw: Vec<(CanonicalAddr, Uint128)> = collaterals
        .iter()
        .map(|c| Ok((deps.api.canonical_address(&c.0)?, c.1)))
        .collect::<StdResult<Vec<(CanonicalAddr, Uint128)>>>()?;

    cur_collaterals.add(collaterals_raw);
    store_collaterals(&mut deps.storage, &borrower_raw, &cur_collaterals)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals.clone() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral.0,
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::LockCollateral {
                borrower: env.message.sender.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Loging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals
        .iter()
        .map(|c| format!("{}{}", c.1, c.0.to_string()))
        .collect();

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "lock_collateral"),
            log("borrower", env.message.sender),
            log("collaterals", collateral_logs.join(",")),
        ],
        data: None,
    })
}

pub fn handle_unlock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collaterals: Vec<(HumanAddr, Uint128)>,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let market = deps.api.human_address(&config.market_contract)?;

    let borrower = env.message.sender;
    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let mut cur_collaterals: Tokens = read_collaterals(&deps.storage, &borrower_raw);

    let collaterals_raw: Tokens = collaterals
        .iter()
        .map(|c| Ok((deps.api.canonical_address(&c.0)?, c.1)))
        .collect::<StdResult<Tokens>>()?;

    // Underflow check is done in sub_collateral
    if !cur_collaterals.sub(collaterals_raw).is_ok() {
        return Err(StdError::generic_err("Cannot unlock more than you have"));
    }

    // Compute borrow limit with collaterals except unlock target collaterals
    let borrow_limit = compute_borrow_limit(deps, &cur_collaterals)?;
    let borrow_amount_res: BorrowAmountResponse = load_borrow_amount(deps, &market, &borrower)?;
    if borrow_limit < borrow_amount_res.amount {
        return Err(StdError::generic_err(
            "Cannot unlock collateral more than minimum LTV",
        ));
    }

    store_collaterals(&mut deps.storage, &borrower_raw, &cur_collaterals)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals.clone() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral.0,
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::UnlockCollateral {
                borrower: borrower.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Loging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals
        .iter()
        .map(|c| format!("{}{}", c.1, c.0.to_string()))
        .collect();

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "unlock_collateral"),
            log("borrower", borrower),
            log("collaterals", collateral_logs.join(",")),
        ],
        data: None,
    })
}

pub fn handle_liquidiate_collateral<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _borrower: HumanAddr,
) -> HandleResult {
    // TODO - implement liquidation
    Ok(HandleResponse::default())
}

pub fn query_collaterals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrower: HumanAddr,
) -> StdResult<CollateralsResponse> {
    let collaterals: Tokens =
        read_collaterals(&deps.storage, &deps.api.canonical_address(&borrower)?);

    Ok(CollateralsResponse {
        borrower,
        collaterals: collaterals
            .iter()
            .map(|c| Ok((deps.api.human_address(&c.0)?, c.1)))
            .collect::<StdResult<Vec<(HumanAddr, Uint128)>>>()?,
    })
}

pub fn query_all_collaterals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<AllCollateralsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.canonical_address(&start_after)?)
    } else {
        None
    };

    let all_collaterals: Vec<CollateralsResponse> =
        read_all_collaterals(&deps, start_after, limit)?;

    Ok(AllCollateralsResponse { all_collaterals })
}

fn compute_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    collaterals: &Tokens,
) -> StdResult<Uint128> {
    let config: Config = read_config(&deps.storage)?;
    let oracle_contract = deps.api.human_address(&config.oracle_contract)?;

    let mut borrow_limit: Uint128 = Uint128::zero();
    for collateral in collaterals.iter() {
        let collateral_token = collateral.0.clone();
        let collateral_amount = collateral.1;

        let price: OraclePriceResponse = load_oracle_price(
            &deps,
            &oracle_contract,
            config.base_denom.to_string(),
            collateral_token.to_string(),
        )?;

        // TODO check price last_updated

        let elem: WhitelistElem = read_whitelist_elem(&deps.storage, &collateral.0)?;
        borrow_limit += collateral_amount * elem.ltv * price.rate;
    }

    Ok(borrow_limit)
}

pub fn query_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrower: HumanAddr,
) -> StdResult<BorrowLimitResponse> {
    let collaterals = read_collaterals(&deps.storage, &deps.api.canonical_address(&borrower)?);

    // Compute borrow limit with collaterals
    let borrow_limit = compute_borrow_limit(deps, &collaterals)?;

    Ok(BorrowLimitResponse {
        borrower,
        borrow_limit,
    })
}
