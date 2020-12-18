use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage,
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
            min_liquidation: msg.min_liquidation,
            liquidation_threshold: msg.liquidation_threshold,
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
            safe_ratio,
            min_liquidation,
            liquidation_threshold,
        } => update_config(
            deps,
            env,
            owner,
            safe_ratio,
            min_liquidation,
            liquidation_threshold,
        ),
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    safe_ratio: Option<Decimal256>,
    min_liquidation: Option<Uint256>,
    liquidation_threshold: Option<Uint256>,
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

    if let Some(min_liquidation) = min_liquidation {
        config.min_liquidation = min_liquidation;
    }

    if let Some(liquidation_threshold) = liquidation_threshold {
        config.liquidation_threshold = liquidation_threshold;
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
            collateral_prices,
        } => to_binary(&query_liquidation_amount(
            deps,
            borrow_amount,
            borrow_limit,
            stable_denom,
            collaterals,
            collateral_prices,
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
        min_liquidation: state.min_liquidation,
        liquidation_threshold: state.liquidation_threshold,
    };

    Ok(resp)
}

fn query_liquidation_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    borrow_amount: Uint256,
    borrow_limit: Uint256,
    _stable_denom: String,
    collaterals: TokensHuman,
    collateral_prices: Vec<Decimal256>,
) -> StdResult<LiquidationAmountResponse> {
    let config: Config = read_config(&deps.storage)?;
    let safe_borrow_limit = borrow_limit * config.safe_ratio;

    // Safely collateralized check
    if borrow_amount <= safe_borrow_limit {
        return Ok(LiquidationAmountResponse {
            collaterals: vec![],
        });
    }

    let mut collaterals_value = Uint256::zero();
    for c in collaterals.iter().zip(collateral_prices.iter()) {
        let (collateral, price) = c;
        let collateral_value = collateral.1 * *price;
        collaterals_value += collateral_value;
    }

    // collaterals_value must be bigger than safe_borrow_limit
    if collaterals_value <= safe_borrow_limit {
        return Err(StdError::generic_err(
            "safe_borrow_limit cannot be smaller than collaterals value",
        ));
    }

    // When collaterals_value is smaller than liquidation_threshold,
    // liquidate all collaterals
    let liquidation_ratio = if collaterals_value < config.liquidation_threshold {
        Decimal256::one()
    } else {
        Decimal256::from_uint256(borrow_amount - safe_borrow_limit)
            / Decimal256::from_uint256(collaterals_value - safe_borrow_limit)
    };

    Ok(LiquidationAmountResponse {
        collaterals: collaterals
            .iter()
            .zip(collateral_prices.iter())
            .map(|c| {
                let (collateral, price) = c;
                let mut collateral = collateral.clone();

                // When target liquidation amount is smaller than `min_liquidation`
                // except from the liquidation
                collateral.1 = collateral.1 * liquidation_ratio;
                if config.min_liquidation > collateral.1 * *price {
                    collateral.1 = Uint256::zero();
                }

                collateral
            })
            .filter(|c| c.1 > Uint256::zero())
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
            safe_ratio: Decimal256::percent(10),
            min_liquidation: Uint256::from(1000000u64),
            liquidation_threshold: Uint256::from(100000000u64),
        };

        let env = mock_env("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!(
            value,
            ConfigResponse {
                owner: HumanAddr::from("owner0000"),
                safe_ratio: Decimal256::percent(10),
                min_liquidation: Uint256::from(1000000u64),
                liquidation_threshold: Uint256::from(100000000u64),
            }
        );
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr("owner0000".to_string()),
            safe_ratio: Decimal256::percent(10),
            min_liquidation: Uint256::from(1000000u64),
            liquidation_threshold: Uint256::from(100000000u64),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // update owner
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner0001".to_string())),
            safe_ratio: None,
            min_liquidation: None,
            liquidation_threshold: None,
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_config(&deps).unwrap();
        assert_eq!(
            value,
            ConfigResponse {
                owner: HumanAddr::from("owner0001"),
                safe_ratio: Decimal256::percent(10),
                min_liquidation: Uint256::from(1000000u64),
                liquidation_threshold: Uint256::from(100000000u64),
            }
        );

        // Unauthorzied err
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            safe_ratio: Some(Decimal256::percent(1)),
            min_liquidation: Some(Uint256::from(10000000u64)),
            liquidation_threshold: Some(Uint256::from(1000000000u64)),
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
            safe_ratio: Decimal256::percent(10),
            min_liquidation: Uint256::from(100000u64),
            liquidation_threshold: Uint256::from(1000000u64),
        };

        let env = mock_env("addr0000", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        let msg = QueryMsg::LiquidationAmount {
            borrow_amount: Uint256::from(1000000u64),
            borrow_limit: Uint256::from(1000000u64),
            stable_denom: "uusd".to_string(),
            collaterals: vec![(HumanAddr::from("token0000"), Uint256::from(1000000u64))],
            collateral_prices: vec![Decimal256::percent(10)],
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
            borrow_amount: Uint256::from(100000u64),
            borrow_limit: Uint256::from(1000000u64),
            stable_denom: "uusd".to_string(),
            collaterals: vec![(HumanAddr::from("token0000"), Uint256::from(1000000u64))],
            collateral_prices: vec![Decimal256::one()],
        };

        let res = query(&mut deps, msg).unwrap();
        let res: LiquidationAmountResponse = from_binary(&res).unwrap();
        assert_eq!(
            res,
            LiquidationAmountResponse {
                collaterals: vec![],
            }
        );

        let query_msg = QueryMsg::LiquidationAmount {
            borrow_amount: Uint256::from(1000000u64),
            borrow_limit: Uint256::from(1000000u64),
            stable_denom: "uusd".to_string(),
            collaterals: vec![
                (HumanAddr::from("token0000"), Uint256::from(1000000u64)),
                (HumanAddr::from("token0001"), Uint256::from(2000000u64)),
                (HumanAddr::from("token0002"), Uint256::from(3000000u64)),
            ],
            collateral_prices: vec![
                Decimal256::percent(50),
                Decimal256::percent(50),
                Decimal256::percent(50),
            ],
        };

        // liquidation_ratio = 0.3103448276
        let res = query(&mut deps, query_msg.clone()).unwrap();
        let res: LiquidationAmountResponse = from_binary(&res).unwrap();
        assert_eq!(
            res,
            LiquidationAmountResponse {
                collaterals: vec![
                    (HumanAddr::from("token0000"), Uint256::from(310344u64)),
                    (HumanAddr::from("token0001"), Uint256::from(620689u64)),
                    (HumanAddr::from("token0002"), Uint256::from(931034u64)),
                ],
            }
        );

        // increase min_liquidation
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            safe_ratio: None,
            min_liquidation: Some(Uint256::from(200000u64)),
            liquidation_threshold: None,
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // token0000 excluded due to min_liquidation
        let res = query(&mut deps, query_msg.clone()).unwrap();
        let res: LiquidationAmountResponse = from_binary(&res).unwrap();
        assert_eq!(
            res,
            LiquidationAmountResponse {
                collaterals: vec![
                    (HumanAddr::from("token0001"), Uint256::from(620689u64)),
                    (HumanAddr::from("token0002"), Uint256::from(931034u64)),
                ],
            }
        );

        // increase min_liquidation
        let env = mock_env("owner0000", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: None,
            safe_ratio: None,
            min_liquidation: None,
            liquidation_threshold: Some(Uint256::from(10000000u64)),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // token0000 excluded due to min_liquidation
        let res = query(&mut deps, query_msg.clone()).unwrap();
        let res: LiquidationAmountResponse = from_binary(&res).unwrap();
        assert_eq!(
            res,
            LiquidationAmountResponse {
                collaterals: vec![
                    (HumanAddr::from("token0000"), Uint256::from(1000000u64)),
                    (HumanAddr::from("token0001"), Uint256::from(2000000u64)),
                    (HumanAddr::from("token0002"), Uint256::from(3000000u64)),
                ],
            }
        );
    }
}
