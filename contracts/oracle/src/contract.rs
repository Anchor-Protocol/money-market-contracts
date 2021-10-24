use crate::error::ContractError;
use crate::state::{
    read_config, read_feeder, read_price, read_prices, store_config, store_feeder, store_price,
    Config, PriceInfo,
};
use cosmwasm_bignumber::Decimal256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use moneymarket::oracle::{
    ConfigResponse, ExecuteMsg, FeederResponse, InstantiateMsg, PriceResponse, PricesResponse,
    PricesResponseElem, QueryMsg,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(&msg.owner)?,
            base_asset: msg.base_asset,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, info, owner),
        ExecuteMsg::RegisterFeeder { asset, feeder } => register_feeder(deps, info, asset, feeder),
        ExecuteMsg::FeedPrice { prices } => feed_prices(deps, env, info, prices),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn register_feeder(
    deps: DepsMut,
    info: MessageInfo,
    asset: String,
    feeder: String,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    store_feeder(deps.storage, &asset, &deps.api.addr_canonicalize(&feeder)?)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_feeder"),
        attr("asset", asset),
        attr("feeder", feeder),
    ]))
}

pub fn feed_prices(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    prices: Vec<(String, Decimal256)>,
) -> Result<Response, ContractError> {
    let mut attributes = vec![attr("action", "feed_prices")];
    let sender_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    for price in prices {
        let asset: String = price.0;
        let price: Decimal256 = price.1;

        // Check feeder permission
        let feeder = read_feeder(deps.storage, &asset)?;
        if feeder != sender_raw {
            return Err(ContractError::Unauthorized {});
        }

        attributes.push(attr("asset", asset.to_string()));
        attributes.push(attr("price", price.to_string()));

        store_price(
            deps.storage,
            &asset,
            &PriceInfo {
                last_updated_time: env.block.time.seconds(),
                price,
            },
        )?;
    }

    Ok(Response::new().add_attributes(attributes))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Feeder { asset } => to_binary(&query_feeder(deps, asset)?),
        QueryMsg::Price { base, quote } => to_binary(&query_price(deps, base, quote)?),
        QueryMsg::Prices { start_after, limit } => {
            to_binary(&query_prices(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        base_asset: state.base_asset,
    };

    Ok(resp)
}

fn query_feeder(deps: Deps, asset: String) -> StdResult<FeederResponse> {
    let feeder = read_feeder(deps.storage, &asset)?;
    let resp = FeederResponse {
        asset,
        feeder: deps.api.addr_humanize(&feeder)?.to_string(),
    };

    Ok(resp)
}

fn query_price(deps: Deps, base: String, quote: String) -> StdResult<PriceResponse> {
    let config: Config = read_config(deps.storage)?;
    let quote_price = if config.base_asset == quote {
        PriceInfo {
            price: Decimal256::one(),
            last_updated_time: 9999999999,
        }
    } else {
        read_price(deps.storage, &quote)?
    };

    let base_price = if config.base_asset == base {
        PriceInfo {
            price: Decimal256::one(),
            last_updated_time: 9999999999,
        }
    } else {
        read_price(deps.storage, &base)?
    };

    Ok(PriceResponse {
        rate: base_price.price / quote_price.price,
        last_updated_base: base_price.last_updated_time,
        last_updated_quote: quote_price.last_updated_time,
    })
}

fn query_prices(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PricesResponse> {
    let prices: Vec<PricesResponseElem> = read_prices(deps.storage, start_after, limit)?;
    Ok(PricesResponse { prices })
}
