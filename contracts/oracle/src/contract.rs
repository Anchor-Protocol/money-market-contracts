use cosmwasm_std::{
    log, to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};

use crate::math::decimal_division;
use crate::msg::{
    ConfigResponse, HandleMsg, InitMsg, PriceResponse, PricesResponse, PricesResponseElem, QueryMsg,
};
use crate::state::{
    read_config, read_price, read_prices, store_config, store_price, Config, PriceInfo,
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: deps.api.canonical_address(&msg.owner)?,
            base_asset: msg.base_asset,
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::UpdateConfig { owner } => update_config(deps, env, owner),
        HandleMsg::FeedPrice { prices } => feed_prices(deps, env, prices),
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse::default())
}

pub fn feed_prices<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    prices: Vec<(String, Decimal)>,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;

    // Check feeder permission
    if config.owner != deps.api.canonical_address(&env.message.sender)? {
        return Err(StdError::unauthorized());
    }

    let mut logs = vec![log("action", "feed_prices")];
    for price in prices {
        let asset: String = price.0;
        let price: Decimal = price.1;

        logs.push(log("asset", asset.to_string()));
        logs.push(log("price", price));

        store_price(
            &mut deps.storage,
            &asset,
            &PriceInfo {
                last_updated_time: env.block.time,
                price,
            },
        )?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: logs,
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Price { base, quote } => to_binary(&query_price(deps, base, quote)?),
        QueryMsg::Prices { start_after, limit } => {
            to_binary(&query_prices(deps, start_after, limit))
        }
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        base_asset: state.base_asset,
    };

    Ok(resp)
}

fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    base: String,
    quote: String,
) -> StdResult<PriceResponse> {
    let config: Config = read_config(&deps.storage)?;
    let quote_price = if config.base_asset == quote {
        PriceInfo {
            price: Decimal::one(),
            last_updated_time: 9999999999,
        }
    } else {
        read_price(&deps.storage, &quote.to_string())?
    };

    let base_price = if config.base_asset == base {
        PriceInfo {
            price: Decimal::one(),
            last_updated_time: 9999999999,
        }
    } else {
        read_price(&deps.storage, &base.to_string())?
    };

    Ok(PriceResponse {
        rate: decimal_division(base_price.price, quote_price.price),
        last_updated_base: base_price.last_updated_time,
        last_updated_quote: quote_price.last_updated_time,
    })
}

fn query_prices<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PricesResponse> {
    let prices: Vec<PricesResponseElem> = read_prices(&deps.storage, start_after, limit)?;
    Ok(PricesResponse { prices })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::StdError;
    use std::str::FromStr;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            base_asset: "base0000".to_string(),
        };

        let env = mock_env("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0000", value.owner.as_str());
        assert_eq!("base0000", &value.base_asset.to_string());
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            base_asset: "base0000".to_string(),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // update owner
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner0001".to_string())),
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0001", value.owner.as_str());
        assert_eq!("base0000", &value.base_asset.to_string());

        // Unauthorzied err
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig { owner: None };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn feed_price() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            base_asset: "base0000".to_string(),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        let msg = HandleMsg::FeedPrice {
            prices: vec![
                ("mAPPL".to_string(), Decimal::from_str("1.2").unwrap()),
                ("mGOGL".to_string(), Decimal::from_str("2.2").unwrap()),
            ],
        };
        let env = mock_env("owner0000", &[]);
        let _res = handle(&mut deps, env.clone(), msg).unwrap();
        let value: PriceResponse =
            query_price(&deps, "mAPPL".to_string(), "base0000".to_string()).unwrap();
        assert_eq!(
            value,
            PriceResponse {
                rate: Decimal::from_str("1.2").unwrap(),
                last_updated_base: env.block.time,
                last_updated_quote: 9999999999,
            }
        );

        let value: PriceResponse =
            query_price(&deps, "mGOGL".to_string(), "mAPPL".to_string()).unwrap();
        assert_eq!(
            value,
            PriceResponse {
                rate: Decimal::from_str("1.833333333333333333").unwrap(),
                last_updated_base: env.block.time,
                last_updated_quote: env.block.time,
            }
        );

        let value: PricesResponse = query_prices(&deps, None, None).unwrap();
        assert_eq!(
            value,
            PricesResponse {
                prices: vec![
                    PricesResponseElem {
                        asset: "mAPPL".to_string(),
                        price: Decimal::from_str("1.2").unwrap(),
                        last_updated_time: env.block.time,
                    },
                    PricesResponseElem {
                        asset: "mGOGL".to_string(),
                        price: Decimal::from_str("2.2").unwrap(),
                        last_updated_time: env.block.time,
                    }
                ],
            }
        );

        // Unautorized try
        let env = mock_env("addr0001", &[]);
        let msg = HandleMsg::FeedPrice {
            prices: vec![("mAPPL".to_string(), Decimal::from_str("1.2").unwrap())],
        };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }
}
