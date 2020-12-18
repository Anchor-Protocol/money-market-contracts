use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier,
    StdError, StdResult, Storage, WasmMsg,
};

use crate::msg::{AllCollateralsResponse, BorrowLimitResponse, CollateralsResponse};
use crate::state::{
    read_all_collaterals, read_collaterals, read_config, read_whitelist_elem, store_collaterals,
    Config, WhitelistElem,
};

use moneymarket::{
    query_balance, query_liquidation_amount, query_loan_amount, query_price, CustodyHandleMsg,
    LiquidationAmountResponse, LoanAmountResponse, MarketHandleMsg, PriceResponse, Tokens,
    TokensHuman, TokensMath, TokensToHuman, TokensToRaw,
};

pub fn lock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collaterals_human: TokensHuman,
) -> HandleResult {
    let borrower_raw = deps.api.canonical_address(&env.message.sender)?;
    let mut cur_collaterals: Tokens = read_collaterals(&deps.storage, &borrower_raw);

    let collaterals: Tokens = collaterals_human.to_raw(&deps)?;

    cur_collaterals.add(collaterals.clone());
    store_collaterals(&mut deps.storage, &borrower_raw, &cur_collaterals)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(&deps.storage, &collateral.0)?;
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&whitelist_elem.custody_contract)?,
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::LockCollateral {
                borrower: env.message.sender.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Loging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals_human
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

pub fn unlock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collaterals_human: TokensHuman,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let market = deps.api.human_address(&config.market_contract)?;

    let borrower = env.message.sender;
    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let mut cur_collaterals: Tokens = read_collaterals(&deps.storage, &borrower_raw);
    let collaterals: Tokens = collaterals_human.to_raw(&deps)?;

    // Underflow check is done in sub_collateral
    if cur_collaterals.sub(collaterals.clone()).is_err() {
        return Err(StdError::generic_err("Cannot unlock more than you have"));
    }

    // Compute borrow limit with collaterals except unlock target collaterals
    let (borrow_limit, _) = compute_borrow_limit(deps, &cur_collaterals)?;
    let borrow_amount_res: LoanAmountResponse =
        query_loan_amount(deps, &market, &borrower, env.block.height)?;
    if borrow_limit < borrow_amount_res.loan_amount {
        return Err(StdError::generic_err(
            "Cannot unlock collateral more than LTV",
        ));
    }

    store_collaterals(&mut deps.storage, &borrower_raw, &cur_collaterals)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals.clone() {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(&deps.storage, &collateral.0)?;
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&whitelist_elem.custody_contract)?,
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::UnlockCollateral {
                borrower: borrower.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Loging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals_human
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

pub fn liquidate_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    borrower: HumanAddr,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let market = deps.api.human_address(&config.market_contract)?;

    let borrower_raw = deps.api.canonical_address(&borrower)?;
    let mut cur_collaterals: Tokens = read_collaterals(&deps.storage, &borrower_raw);

    // Compute borrow limit with collaterals except unlock target collaterals
    let (borrow_limit, collateral_prices) = compute_borrow_limit(deps, &cur_collaterals)?;
    let borrow_amount_res: LoanAmountResponse =
        query_loan_amount(deps, &market, &borrower, env.block.height)?;
    let borrow_amount = borrow_amount_res.loan_amount;

    // borrow limit is equal or bigger than loan amount
    // cannot liquidation collaterals
    if borrow_limit >= borrow_amount {
        return Err(StdError::generic_err(
            "Cannot liquidate safely collateralized borrower",
        ));
    }

    let liquidation_amount_res: LiquidationAmountResponse = query_liquidation_amount(
        &deps,
        &deps.api.human_address(&config.liquidation_model)?,
        borrow_amount,
        borrow_limit,
        config.stable_denom.to_string(),
        &cur_collaterals.to_human(&deps)?,
        collateral_prices,
    )?;

    let liquidation_amount = liquidation_amount_res.collaterals.to_raw(&deps)?;

    // Store left collaterals
    cur_collaterals.sub(liquidation_amount.clone())?;
    store_collaterals(&mut deps.storage, &borrower_raw, &cur_collaterals)?;

    let market_contract = deps.api.human_address(&config.market_contract)?;
    let prev_balance: Uint256 = query_balance(&deps, &market_contract, config.stable_denom)?;

    let liquidation_messages: Vec<CosmosMsg> = liquidation_amount
        .iter()
        .map(|collateral| {
            let whitelist_elem: WhitelistElem = read_whitelist_elem(&deps.storage, &collateral.0)?;

            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.human_address(&whitelist_elem.custody_contract)?,
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::LiquidateCollateral {
                    borrower: borrower.clone(),
                    amount: collateral.1,
                })?,
            }))
        })
        .filter(|msg| msg.is_ok())
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(HandleResponse {
        messages: vec![
            liquidation_messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: market_contract,
                send: vec![],
                msg: to_binary(&MarketHandleMsg::RepayStableFromLiquidation {
                    borrower,
                    prev_balance,
                })?,
            })],
        ]
        .concat(),
        log: vec![],
        data: None,
    })
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
            .collect::<StdResult<Vec<(HumanAddr, Uint256)>>>()?,
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

#[allow(clippy::ptr_arg)]
fn compute_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    collaterals: &Tokens,
) -> StdResult<(Uint256, Vec<Decimal256>)> {
    let config: Config = read_config(&deps.storage)?;
    let oracle_contract = deps.api.human_address(&config.oracle_contract)?;

    let mut borrow_limit: Uint256 = Uint256::zero();
    let mut collateral_prices: Vec<Decimal256> = vec![];
    for collateral in collaterals.iter() {
        let collateral_token = collateral.0.clone();
        let collateral_amount = collateral.1;

        let price: PriceResponse = query_price(
            &deps,
            &oracle_contract,
            (deps.api.human_address(&collateral_token)?).to_string(),
            config.stable_denom.to_string(),
        )?;

        // TODO check price last_updated

        let elem: WhitelistElem = read_whitelist_elem(&deps.storage, &collateral.0)?;
        let collateral_value = collateral_amount * price.rate;
        borrow_limit += collateral_value * elem.ltv;
        collateral_prices.push(price.rate);
    }

    // returns borrow_limit with collaterals value in stable denom
    Ok((borrow_limit, collateral_prices))
}

pub fn query_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrower: HumanAddr,
) -> StdResult<BorrowLimitResponse> {
    let collaterals = read_collaterals(&deps.storage, &deps.api.canonical_address(&borrower)?);

    // Compute borrow limit with collaterals
    let (borrow_limit, _) = compute_borrow_limit(deps, &collaterals)?;

    Ok(BorrowLimitResponse {
        borrower,
        borrow_limit,
    })
}
