use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{to_binary, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use moneymarket::overseer::{QueryMsg as OverseerQueryMsg, WhitelistResponse};

pub fn query_collateral_max_ltv(
    querier: &QuerierWrapper,
    overseer: String,
    collateral_token: String,
) -> StdResult<Decimal256> {
    let whitelist_elem: WhitelistResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: overseer,
            msg: to_binary(&OverseerQueryMsg::Whitelist {
                collateral_token: Some(collateral_token),
                start_after: None,
                limit: None,
            })?,
        }))?;

    Ok(whitelist_elem.elems[0].max_ltv)
}
