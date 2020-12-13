use crate::collateral::compute_borrow_limit;
use crate::contract::{handle, init};
use crate::msg::{HandleMsg, InitMsg};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Api, Decimal, HumanAddr, Uint128};
use moneymarket::{Token, Tokens};

#[test]
fn proper_compute_borrow_limit() {
    let mut deps = mock_dependencies(20, &[]);

    let env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_model: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        distribution_threshold: Decimal::permille(3),
        target_deposit_rate: Decimal::permille(5),
        buffer_distribution_rate: Decimal::percent(20),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        ltv: Decimal::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        ltv: Decimal::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    deps.querier.with_oracle_price(&[
        (
            &("bluna".to_string(), "uusd".to_string()),
            &(
                Decimal::from_ratio(1000u128, 1u128),
                env.block.time,
                env.block.time,
            ),
        ),
        (
            &("batom".to_string(), "uusd".to_string()),
            &(
                Decimal::from_ratio(2000u128, 1u128),
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
        Uint128(1000),
    );
    collaterals.push(token1);
    let token2: Token = (
        deps.api
            .canonical_address(&HumanAddr::from("batom"))
            .unwrap(),
        Uint128(1000),
    );
    collaterals.push(token2);

    let res = compute_borrow_limit(&deps, &collaterals).unwrap();
    let mut vec: Vec<Decimal> = vec![];
    vec.push(Decimal::from_ratio(Uint128(1000), Uint128(1)));
    vec.push(Decimal::from_ratio(Uint128(2000), Uint128(1)));

    let res2 = (Uint128(1800000), vec);
    assert_eq!(res, res2);
}
