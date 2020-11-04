use cosmwasm_std::{to_binary, Api, Binary, Extern, Querier, StdResult, Storage};

use crate::borrow::{query_liability, query_liabilitys, query_loan_amount};
use crate::msg::{ConfigResponse, QueryMsg};
use crate::state::{read_config, read_state, Config, State};

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Liability { borrower } => to_binary(&query_liability(deps, borrower)?),
        QueryMsg::Liabilitys { start_after, limit } => {
            to_binary(&query_liabilitys(deps, start_after, limit)?)
        }
        QueryMsg::LoanAmount { borrower } => to_binary(&query_loan_amount(deps, borrower)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config: Config = read_config(&deps.storage)?;
    Ok(ConfigResponse {
        owner_addr: deps.api.human_address(&config.owner_addr)?,
        anchor_token: deps.api.human_address(&config.anchor_token)?,
        interest_model: deps.api.human_address(&config.interest_model)?,
        overseer_contract: deps.api.human_address(&config.overseer_contract)?,
        base_denom: config.base_denom,
        reserve_factor: config.reserve_factor,
    })
}

pub fn query_state<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<State> {
    let state: State = read_state(&deps.storage)?;
    Ok(state)
}
