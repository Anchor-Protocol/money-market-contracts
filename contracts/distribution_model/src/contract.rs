use crate::state::{read_config, store_config, Config};

use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage,
};
use moneymarket::distribution_model::{
    AncEmissionRateResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg,
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
            emission_cap: msg.emission_cap,
            increment_multiplier: msg.increment_multiplier,
            decrement_multiplier: msg.decrement_multiplier,
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
            emission_cap,
            increment_multiplier,
            decrement_multiplier,
        } => update_config(
            deps,
            env,
            owner,
            emission_cap,
            increment_multiplier,
            decrement_multiplier,
        ),
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    emission_cap: Option<Decimal256>,
    increment_multiplier: Option<Decimal256>,
    decrement_multiplier: Option<Decimal256>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(emission_cap) = emission_cap {
        config.emission_cap = emission_cap;
    }

    if let Some(increment_multiplier) = increment_multiplier {
        config.increment_multiplier = increment_multiplier;
    }

    if let Some(decrement_multiplier) = decrement_multiplier {
        config.decrement_multiplier = decrement_multiplier;
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
        QueryMsg::AncEmissionRate {
            deposit_rate,
            target_deposit_rate,
            current_emission_rate,
        } => to_binary(&query_anc_emission_rate(
            deps,
            deposit_rate,
            target_deposit_rate,
            current_emission_rate,
        )?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        emission_cap: state.emission_cap,
        increment_multiplier: state.increment_multiplier,
        decrement_multiplier: state.decrement_multiplier,
    };

    Ok(resp)
}

fn query_anc_emission_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    deposit_rate: Decimal256,
    target_deposit_rate: Decimal256,
    current_emission_rate: Decimal256,
) -> StdResult<AncEmissionRateResponse> {
    let config: Config = read_config(&deps.storage)?;

    let emission_rate = if deposit_rate < target_deposit_rate {
        current_emission_rate * config.increment_multiplier
    } else if deposit_rate > target_deposit_rate {
        current_emission_rate * config.decrement_multiplier
    } else {
        current_emission_rate
    };

    let emission_rate = if emission_rate > config.emission_cap {
        config.emission_cap
    } else {
        emission_rate
    };

    Ok(AncEmissionRateResponse { emission_rate })
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
            emission_cap: Decimal256::from_uint256(100u64),
            increment_multiplier: Decimal256::percent(110),
            decrement_multiplier: Decimal256::percent(90),
        };

        let env = mock_env("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0000", value.owner.as_str());
        assert_eq!("100", &value.emission_cap.to_string());
        assert_eq!("1.1", &value.increment_multiplier.to_string());
        assert_eq!("0.9", &value.decrement_multiplier.to_string());

        let value = query_anc_emission_rate(
            &deps,
            Decimal256::percent(10),
            Decimal256::percent(10),
            Decimal256::from_uint256(99u128),
        )
        .unwrap();
        assert_eq!("99", &value.emission_rate.to_string());

        let value = query_anc_emission_rate(
            &deps,
            Decimal256::percent(10),
            Decimal256::percent(12),
            Decimal256::from_uint256(99u128),
        )
        .unwrap();
        assert_eq!("100", &value.emission_rate.to_string());

        let value = query_anc_emission_rate(
            &deps,
            Decimal256::percent(15),
            Decimal256::percent(12),
            Decimal256::from_uint256(99u128),
        )
        .unwrap();
        assert_eq!("89.1", &value.emission_rate.to_string());
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            emission_cap: Decimal256::from_uint256(100u64),
            increment_multiplier: Decimal256::percent(110),
            decrement_multiplier: Decimal256::percent(90),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // update owner
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner0001".to_string())),
            emission_cap: None,
            increment_multiplier: None,
            decrement_multiplier: None,
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0001", value.owner.as_str());
        assert_eq!("100", &value.emission_cap.to_string());
        assert_eq!("1.1", &value.increment_multiplier.to_string());
        assert_eq!("0.9", &value.decrement_multiplier.to_string());

        // Unauthorized err
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            emission_cap: Some(Decimal256::from_uint256(100u64)),
            increment_multiplier: Some(Decimal256::percent(110)),
            decrement_multiplier: Some(Decimal256::percent(90)),
        };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }
}
