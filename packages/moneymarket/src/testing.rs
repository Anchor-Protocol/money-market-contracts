use crate::mock_querier::mock_dependencies;
use crate::querier::{
    compute_tax, deduct_tax, load_all_balances, load_balance, load_distribution_params,
    load_epoch_state, load_oracle_price, load_supply, load_token_balance,
    DistributionParamsResponse, EpochStateResponse, OraclePriceResponse,
};
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{Coin, Decimal, HumanAddr, Uint128};

#[test]
fn token_balance_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("liquidity0000"),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(123u128))],
    )]);

    assert_eq!(
        Uint128(123u128),
        load_token_balance(
            &deps,
            &HumanAddr::from("liquidity0000"),
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
        )
        .unwrap()
    );
}

#[test]
fn balance_querier() {
    let deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(200u128),
        }],
    );

    assert_eq!(
        load_balance(
            &deps,
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            "uusd".to_string()
        )
        .unwrap(),
        Uint128(200u128)
    );
}

#[test]
fn all_balances_querier() {
    let deps = mock_dependencies(
        20,
        &[
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128(100u128),
            },
        ],
    );

    assert_eq!(
        load_all_balances(&deps, &HumanAddr::from(MOCK_CONTRACT_ADDR),).unwrap(),
        vec![
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128(100u128),
            }
        ]
    );
}

#[test]
fn a_value_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_distribution_params(&[(
        &HumanAddr::from("asset0000"),
        &(Decimal::percent(1), Decimal::percent(2)),
    )]);

    assert_eq!(
        load_distribution_params(
            &deps,
            &HumanAddr::from("overseer"),
            &HumanAddr::from("asset0000")
        )
        .unwrap(),
        DistributionParamsResponse {
            deposit_rate: Decimal::percent(1),
            target_deposit_rate: Decimal::percent(2),
        }
    );
}

#[test]
fn supply_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("liquidity0000"),
        &[
            (&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(123u128)),
            (&HumanAddr::from("addr00000"), &Uint128(123u128)),
            (&HumanAddr::from("addr00001"), &Uint128(123u128)),
            (&HumanAddr::from("addr00002"), &Uint128(123u128)),
        ],
    )]);

    assert_eq!(
        load_supply(&deps, &HumanAddr::from("liquidity0000")).unwrap(),
        Uint128(492u128)
    )
}

#[test]
fn test_compute_tax() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    // cap to 1000000
    assert_eq!(
        compute_tax(&deps, &Coin::new(10000000000u128, "uusd")).unwrap(),
        Uint128(1000000u128)
    );

    // normal tax
    assert_eq!(
        compute_tax(&deps, &Coin::new(50000000u128, "uusd")).unwrap(),
        Uint128(495050u128)
    );
}

#[test]
fn test_deduct_tax() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    // cap to 1000000
    assert_eq!(
        deduct_tax(&deps, Coin::new(10000000000u128, "uusd")).unwrap(),
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128(9999000000u128)
        }
    );

    // normal tax
    assert_eq!(
        deduct_tax(&deps, Coin::new(50000000u128, "uusd")).unwrap(),
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128(49504950u128)
        }
    );
}

#[test]
fn test_epoch_state_query() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_epoch_state(&[(
        &HumanAddr::from("market"),
        &(Uint128::from(100u128), Decimal::percent(53)),
    )]);

    let epoch_state = load_epoch_state(&deps, &HumanAddr::from("market")).unwrap();
    assert_eq!(
        epoch_state,
        EpochStateResponse {
            a_token_supply: Uint128::from(100u128),
            exchange_rate: Decimal::percent(53),
        }
    );
}

#[test]
fn test_oracle_price_query() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_oracle_price(&[(
        &("uusd".to_string(), "terra123123".to_string()),
        &(Decimal::from_ratio(131u128, 2u128), 123, 321),
    )]);

    let oracle_price = load_oracle_price(
        &deps,
        &HumanAddr::from("oracle"),
        "uusd".to_string(),
        "terra123123".to_string(),
    )
    .unwrap();

    assert_eq!(
        oracle_price,
        OraclePriceResponse {
            rate: Decimal::from_ratio(131u128, 2u128),
            last_updated_base: 123,
            last_updated_quote: 321,
        }
    );

    load_oracle_price(
        &deps,
        &HumanAddr::from("oracle"),
        "ukrw".to_string(),
        "terra123123".to_string(),
    )
    .unwrap_err();
}
