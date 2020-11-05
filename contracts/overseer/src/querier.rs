use cosmwasm_std::{to_binary, Api, Binary, Extern, HumanAddr, Querier, StdResult, Storage};

use crate::collateral::{query_all_collaterals, query_borrow_limit, query_collaterals};
use crate::msg::{
    ConfigResponse, DistributionParamsResponse, QueryMsg, WhitelistResponse, WhitelistResponseElem,
};
use crate::state::{
    read_config, read_epoch_state, read_whitelist, read_whitelist_elem, Config, EpochState,
    WhitelistElem,
};

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
        base_denom: config.base_denom,
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
    })
}
