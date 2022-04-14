use crate::state::{read_config, read_state, Config, State};
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{to_binary, Addr, Deps, Env, QueryRequest, StdError, StdResult, WasmQuery};
use cw20::{Cw20QueryMsg, TokenInfoResponse};
use moneymarket::ve_aterra::{ConfigResponse, StateResponse};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        contract_addr: deps.api.addr_humanize(&config.contract_addr)?.to_string(),
        owner_addr: deps.api.addr_humanize(&config.owner_addr)?.to_string(),
        market_addr: deps.api.addr_humanize(&config.market_addr)?.to_string(),
        overseer_addr: deps.api.addr_humanize(&config.overseer_addr)?.to_string(),
        aterra_contract: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
        ve_aterra_contract: deps
            .api
            .addr_humanize(&config.ve_aterra_contract)?
            .to_string(),
        max_pos_change: config.max_pos_change,
        max_neg_change: config.max_neg_change,
        max_rate: config.max_rate,
        min_rate: config.min_rate,
        diff_multiplier: config.diff_multiplier,
        premium_rate_epoch: config.premium_rate_epoch,
    })
}

pub fn query_state(deps: Deps, env: Env, block_height: Option<u64>) -> StdResult<StateResponse> {
    let block_height = block_height.unwrap_or(env.block.height);
    let state: State = read_state(deps.storage)?;

    if block_height < state.last_updated {
        return Err(StdError::generic_err(
            "block_height must bigger than last_updated",
        ));
    }

    Ok(StateResponse {})
}

pub fn query_supply(deps: Deps, cw20_contract_addr: Addr) -> StdResult<Uint256> {
    let token_info: TokenInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cw20_contract_addr.to_string(),
            msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;

    Ok(Uint256::from(token_info.total_supply))
}
