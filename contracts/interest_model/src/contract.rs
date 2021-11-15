use crate::error::ContractError;
use crate::state::{read_config, store_config, Config};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_bignumber::Decimal256;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use moneymarket::common::optional_addr_validate;
use moneymarket::interest_model::{
    BorrowRateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg
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
            base_rate: msg.base_rate,
            interest_multiplier: msg.interest_multiplier,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            base_rate,
            interest_multiplier,
        } => {
            let api = deps.api;
            update_config(
                deps,
                info,
                optional_addr_validate(api, owner)?,
                base_rate,
                interest_multiplier,
            )
        }
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    base_rate: Option<Decimal256>,
    interest_multiplier: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(owner.as_str())?;
    }

    if let Some(base_rate) = base_rate {
        config.base_rate = base_rate;
    }

    if let Some(interest_multiplier) = interest_multiplier {
        config.interest_multiplier = interest_multiplier;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::BorrowRate {
            market_balance,
            total_liabilities,
            total_reserves,
        } => to_binary(&query_borrow_rate(
            deps,
            market_balance,
            total_liabilities,
            total_reserves,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        base_rate: state.base_rate,
        interest_multiplier: state.interest_multiplier,
    };

    Ok(resp)
}

fn query_borrow_rate(
    deps: Deps,
    market_balance: Uint256,
    total_liabilities: Decimal256,
    total_reserves: Decimal256,
) -> StdResult<BorrowRateResponse> {
    let config: Config = read_config(deps.storage)?;

    // ignore decimal parts
    let total_value_in_market =
        Decimal256::from_uint256(market_balance) + total_liabilities - total_reserves;

    let utilization_ratio = if total_value_in_market.is_zero() {
        Decimal256::zero()
    } else {
        total_liabilities / total_value_in_market
    };

    Ok(BorrowRateResponse {
        rate: utilization_ratio * config.interest_multiplier + config.base_rate,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    // read and store the new config values
    let mut config = read_config(deps.storage)?;
    config.base_rate = msg.base_rate;
    config.interest_multiplier = msg.interest_multiplier;
    store_config(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use std::str::FromStr;

    #[test]
    fn proper_migrate() {
        let mut deps = mock_dependencies(&[]);

        // init the contract
        let init_msg = InstantiateMsg {
            owner: "owner".to_string(),
            base_rate: Default::default(),
            interest_multiplier: Default::default()
        };

        let info = mock_info("sender", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let base_rate = Decimal256::from_str("1.1").unwrap();
        let interest_multiplier = Decimal256::from_str("2").unwrap();

        // migrate
        let migrate_msg = MigrateMsg {
            base_rate,
            interest_multiplier
        };
        let res = migrate(deps.as_mut(), mock_env(), migrate_msg).unwrap();
        assert_eq!(res, Response::default());

        let config = read_config(&deps.storage).unwrap();
        assert_eq!(config.base_rate, base_rate);
        assert_eq!(config.interest_multiplier, interest_multiplier);
    }
}
