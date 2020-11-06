use cosmwasm_std::{
    to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};

use crate::msg::{BorrowRateResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{read_config, store_config, Config};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: deps.api.canonical_address(&msg.owner)?,
            borrow_rate: msg.borrow_rate,
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
        HandleMsg::UpdateConfig { owner, borrow_rate } => {
            update_config(deps, env, owner, borrow_rate)
        }
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    borrow_rate: Option<Decimal>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(borrow_rate) = borrow_rate {
        config.borrow_rate = borrow_rate;
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
        QueryMsg::BorrowRate {} => to_binary(&query_borrow_rate(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        borrow_rate: state.borrow_rate,
    };

    Ok(resp)
}

fn query_borrow_rate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<BorrowRateResponse> {
    let config: Config = read_config(&deps.storage)?;
    Ok(BorrowRateResponse {
        rate: config.borrow_rate,
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
            borrow_rate: Decimal::percent(10),
        };

        let env = mock_env("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0000", value.owner.as_str());
        assert_eq!("0.1", &value.borrow_rate.to_string());

        let value = query_borrow_rate(&deps).unwrap();
        assert_eq!("0.1", &value.rate.to_string());
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            borrow_rate: Decimal::percent(10),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // update owner
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner0001".to_string())),
            borrow_rate: None,
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0001", value.owner.as_str());
        assert_eq!("0.1", &value.borrow_rate.to_string());

        // Unauthorzied err
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            borrow_rate: Some(Decimal::percent(1)),
        };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }
}
