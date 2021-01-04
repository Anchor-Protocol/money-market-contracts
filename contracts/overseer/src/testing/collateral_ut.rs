use crate::collateral::compute_borrow_limit;
use crate::contract::{handle, init};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Api, HumanAddr};

use moneymarket::overseer::{HandleMsg, InitMsg};
use moneymarket::tokens::{Token, Tokens};

#[test]
fn proper_compute_borrow_limit() {
    let mut deps = mock_dependencies(20, &[]);

    let env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        distribution_threshold: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_rate: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    deps.querier.with_oracle_price(&[
        (
            &("bluna".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_uint256(1000u128),
                env.block.time,
                env.block.time,
            ),
        ),
        (
            &("batom".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_uint256(2000u128),
                env.block.time,
                env.block.time,
            ),
        ),
    ]);

    let mut collaterals: Tokens = vec![];
    let token1: Token = (
        deps.api
            .canonical_address(&HumanAddr::from("bluna"))
            .unwrap(),
        Uint256::from(1000u128),
    );
    collaterals.push(token1);
    let token2: Token = (
        deps.api
            .canonical_address(&HumanAddr::from("batom"))
            .unwrap(),
        Uint256::from(1000u128),
    );
    collaterals.push(token2);

    let res = compute_borrow_limit(&deps, &collaterals, None).unwrap();
    let mut vec: Vec<Decimal256> = vec![];
    vec.push(Decimal256::from_uint256(1000u128));
    vec.push(Decimal256::from_uint256(2000u128));

    let res2 = (Uint256::from(1800000u128), vec);
    assert_eq!(res, res2);
}
