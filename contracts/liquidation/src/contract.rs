use cosmwasm_std::{
    to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage, Uint128,
};

use crate::msg::{ConfigResponse, HandleMsg, InitMsg, LiquidationAmountResponse, QueryMsg};
use crate::state::{read_config, store_config, Config};

use moneymarket::TokensHuman;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: deps.api.canonical_address(&msg.owner)?,
            safe_ratio: msg.safe_ratio,
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
        HandleMsg::UpdateConfig { owner, safe_ratio } => {
            update_config(deps, env, owner, safe_ratio)
        }
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    safe_ratio: Option<Decimal>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if deps.api.canonical_address(&env.message.sender)? != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(safe_ratio) = safe_ratio {
        config.safe_ratio = safe_ratio;
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
        QueryMsg::LiquidationAmount {
            borrow_amount,
            borrow_limit,
            stable_denom,
            collaterals,
            collaterals_amount,
        } => to_binary(&query_liquidation_amount(
            deps,
            borrow_amount,
            borrow_limit,
            stable_denom,
            collaterals,
            collaterals_amount,
        )?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        safe_ratio: state.safe_ratio,
    };

    Ok(resp)
}

fn query_liquidation_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrow_amount: Uint128,
    borrow_limit: Uint128,
    _stable_denom: String,
    collaterals: TokensHuman,
    collaterals_amount: Uint128,
) -> StdResult<LiquidationAmountResponse> {
    let config: Config = read_config(&deps.storage)?;
    let safe_borrow_limit = borrow_limit * config.safe_ratio;

    // Safely collateralized check
    if borrow_amount <= safe_borrow_limit {
        return Ok(LiquidationAmountResponse {
            collaterals: vec![],
        });
    }

    // Check one of borrow_limit, collaterals_amount, safe_ratio
    if collaterals_amount <= safe_borrow_limit {
        return Err(StdError::generic_err(
            "safe_borrow_limit cannot be smaller than collaterals value",
        ));
    }

    let liquidation_ratio = Decimal::from_ratio(
        (borrow_amount - safe_borrow_limit).unwrap(),
        (collaterals_amount - safe_borrow_limit).unwrap(),
    );

    Ok(LiquidationAmountResponse {
        collaterals: collaterals
            .iter()
            .map(|c| {
                let mut c = c.clone();
                c.1 = c.1 * liquidation_ratio;
                c
            })
            .filter(|c| c.1 > Uint128::zero())
            .collect::<TokensHuman>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            safe_ratio: Decimal::percent(10),
        };

        let env = mock_env("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0000", value.owner.as_str());
        assert_eq!("0.1", &value.safe_ratio.to_string());
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            safe_ratio: Decimal::percent(10),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // update owner
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner0001".to_string())),
            safe_ratio: None,
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!("owner0001", value.owner.as_str());
        assert_eq!("0.1", &value.safe_ratio.to_string());

        // Unauthorzied err
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            safe_ratio: Some(Decimal::percent(1)),
        };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn query_liquidation_amount() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            safe_ratio: Decimal::percent(10),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        let msg = QueryMsg::LiquidationAmount {
            borrow_amount: Uint128(1000000u128),
            borrow_limit: Uint128(1000000u128),
            stable_denom: "uusd".to_string(),
            collaterals: vec![],
            collaterals_amount: Uint128(100000u128),
        };

        let res = query(&mut deps, msg);
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(
                msg,
                "safe_borrow_limit cannot be smaller than collaterals value"
            ),
            _ => panic!("DO NOT ENTER HERE"),
        }

        let msg = QueryMsg::LiquidationAmount {
            borrow_amount: Uint128(100000u128),
            borrow_limit: Uint128(1000000u128),
            stable_denom: "uusd".to_string(),
            collaterals: vec![],
            collaterals_amount: Uint128(1000000u128),
        };

        let res = query(&mut deps, msg).unwrap();
        let res: LiquidationAmountResponse = from_binary(&res).unwrap();
        assert_eq!(
            res,
            LiquidationAmountResponse {
                collaterals: vec![],
            }
        );

        let msg = QueryMsg::LiquidationAmount {
            borrow_amount: Uint128(1000000u128),
            borrow_limit: Uint128(1000000u128),
            stable_denom: "uusd".to_string(),
            collaterals: vec![
                (HumanAddr::from("token0000"), Uint128::from(1000000u128)),
                (HumanAddr::from("token0001"), Uint128::from(2000000u128)),
                (HumanAddr::from("token0002"), Uint128::from(3000000u128)),
            ],
            collaterals_amount: Uint128(2000000u128),
        };

        // liquidation_ratio = 0.4736842105
        let res = query(&mut deps, msg).unwrap();
        let res: LiquidationAmountResponse = from_binary(&res).unwrap();
        assert_eq!(
            res,
            LiquidationAmountResponse {
                collaterals: vec![
                    (HumanAddr::from("token0000"), Uint128::from(473684u128)),
                    (HumanAddr::from("token0001"), Uint128::from(947368u128)),
                    (HumanAddr::from("token0002"), Uint128::from(1421052u128)),
                ],
            }
        );
    }
}
