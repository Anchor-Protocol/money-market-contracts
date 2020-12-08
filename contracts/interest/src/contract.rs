use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage, Uint128,
};

use crate::msg::{BorrowRateResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{read_config, store_config, Config};
use cosmwasm_bignumber::Decimal256;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: deps.api.canonical_address(&msg.owner)?,
            base_rate: msg.base_rate,
            interest_multiplier: msg.interest_multiplier,
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
        HandleMsg::UpdateConfig {
            owner,
            base_rate,
            interest_multiplier,
        } => update_config(deps, env, owner, base_rate, interest_multiplier),
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    base_rate: Option<Decimal256>,
    interest_multiplier: Option<Decimal256>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(base_rate) = base_rate {
        config.base_rate = base_rate;
    }

    if let Some(interest_multiplier) = interest_multiplier {
        config.interest_multiplier = interest_multiplier;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::BorrowRate {
            market_balance,
            total_liabilities,
            total_reserve,
        } => to_binary(&query_borrow_rate(
            deps,
            market_balance,
            total_liabilities,
            total_reserve,
        )?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        base_rate: state.base_rate,
        interest_multiplier: state.interest_multiplier,
    };

    Ok(resp)
}

fn query_borrow_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    market_balance: Uint128,
    total_liabilities: Decimal256,
    total_reserve: Decimal256,
) -> StdResult<BorrowRateResponse> {
    let config: Config = read_config(&deps.storage)?;

    // ignore decimal parts
    let total_value_in_market =
        Decimal256::from_uint256(market_balance) + total_liabilities - total_reserve;

    let utilization_ratio = if total_value_in_market.is_zero() {
        Decimal256::zero()
    } else {
        total_liabilities / total_value_in_market
    };

    Ok(BorrowRateResponse {
        rate: utilization_ratio * config.interest_multiplier + config.base_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::StdError;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            base_rate: Decimal256::percent(10),
            interest_multiplier: Decimal256::percent(10),
        };

        let env = mock_env("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0000", value.owner.as_str());
        assert_eq!("0.1", &value.base_rate.to_string());
        assert_eq!("0.1", &value.interest_multiplier.to_string());

        let value = query_borrow_rate(
            &deps,
            Uint128::from(1000000u128),
            Decimal256::from_uint256(500000u128),
            Decimal256::from_uint256(100000u128),
        )
        .unwrap();
        // utilization_ratio = 0.35714285
        // borrow_rate = 0.035714285 + 0.1
        assert_eq!("0.135714285", &value.rate.to_string());

        let value = query_borrow_rate(
            &deps,
            Uint128::zero(),
            Decimal256::zero(),
            Decimal256::zero(),
        )
        .unwrap();
        assert_eq!("0.1", &value.rate.to_string());
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            base_rate: Decimal256::percent(10),
            interest_multiplier: Decimal256::percent(10),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // update owner
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner0001".to_string())),
            base_rate: None,
            interest_multiplier: None,
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0001", value.owner.as_str());
        assert_eq!("0.1", &value.base_rate.to_string());
        assert_eq!("0.1", &value.interest_multiplier.to_string());

        // Unauthorzied err
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            base_rate: Some(Decimal256::percent(1)),
            interest_multiplier: Some(Decimal256::percent(1)),
        };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }
}
