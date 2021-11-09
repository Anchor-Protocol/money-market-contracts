use cosmwasm_std::{to_binary, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use moneymarket::overseer::{
    QueryMsg as OverseerQueryMsg, WhitelistResponse, WhitelistResponseElem,
};

pub fn query_collateral_whitelist_info(
    querier: &QuerierWrapper,
    overseer: String,
    collateral_token: String,
) -> StdResult<WhitelistResponseElem> {
    let whitelist_res: WhitelistResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: overseer,
            msg: to_binary(&OverseerQueryMsg::Whitelist {
                collateral_token: Some(collateral_token),
                start_after: None,
                limit: None,
            })?,
        }))?;

    Ok(whitelist_res.elems[0].clone())
}
