use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{
    to_binary, Api, Extern, HumanAddr, Querier, QueryRequest, StdResult, Storage, WasmQuery,
};
use moneymarket::overseer::{QueryMsg as OverseerQueryMsg, WhitelistResponse};

pub fn query_collateral_max_ltv<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    overseer: &HumanAddr,
    collateral_token: &HumanAddr,
) -> StdResult<Decimal256> {
    let whitelist_elem: WhitelistResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: HumanAddr::from(overseer),
            msg: to_binary(&OverseerQueryMsg::Whitelist {
                collateral_token: Some(HumanAddr::from(collateral_token)),
                start_after: None,
                limit: None,
            })?,
        }))?;

    Ok(whitelist_elem.elems[0].max_ltv)
}
