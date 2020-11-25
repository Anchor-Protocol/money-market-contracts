use crate::mock_querier::mock_dependencies;
use crate::querier::{
    compute_tax, deduct_tax, query_borrow_limit, query_borrow_rate, query_distribution_params,
    query_epoch_state, query_liquidation_amount, query_loan_amount, query_price,
    BorrowLimitResponse, BorrowRateResponse, DistributionParamsResponse, EpochStateResponse,
    LiquidationAmountResponse, LoanAmountResponse, PriceResponse,
};
use cosmwasm_std::{Coin, Decimal, HumanAddr, Uint128};

#[test]
fn distribution_param_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_distribution_params(&[(
        &HumanAddr::from("overseer"),
        &(Decimal::percent(1), Decimal::percent(2)),
    )]);

    assert_eq!(
        query_distribution_params(&deps, &HumanAddr::from("overseer"),).unwrap(),
        DistributionParamsResponse {
            deposit_rate: Decimal::percent(1),
            target_deposit_rate: Decimal::percent(2),
        }
    );
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
fn epoch_state_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_epoch_state(&[(
        &HumanAddr::from("market"),
        &(Uint128::from(100u128), Decimal::percent(53)),
    )]);

    let epoch_state = query_epoch_state(&deps, &HumanAddr::from("market")).unwrap();
    assert_eq!(
        epoch_state,
        EpochStateResponse {
            a_token_supply: Uint128::from(100u128),
            exchange_rate: Decimal::percent(53),
        }
    );
}

#[test]
fn borrow_amount_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier
        .with_loan_amount(&[(&HumanAddr::from("addr0000"), &Uint128::from(100u128))]);

    let borrow_amount = query_loan_amount(
        &deps,
        &HumanAddr::from("market"),
        &HumanAddr::from("addr0000"),
        100u64,
    )
    .unwrap();

    assert_eq!(
        borrow_amount,
        LoanAmountResponse {
            borrower: HumanAddr::from("addr0000"),
            loan_amount: Uint128::from(100u128),
        }
    );
}

#[test]
fn oracle_price_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_oracle_price(&[(
        &("uusd".to_string(), "terra123123".to_string()),
        &(Decimal::from_ratio(131u128, 2u128), 123, 321),
    )]);

    let oracle_price = query_price(
        &deps,
        &HumanAddr::from("oracle"),
        "uusd".to_string(),
        "terra123123".to_string(),
    )
    .unwrap();

    assert_eq!(
        oracle_price,
        PriceResponse {
            rate: Decimal::from_ratio(131u128, 2u128),
            last_updated_base: 123,
            last_updated_quote: 321,
        }
    );

    query_price(
        &deps,
        &HumanAddr::from("oracle"),
        "ukrw".to_string(),
        "terra123123".to_string(),
    )
    .unwrap_err();
}

#[test]
fn borrow_rate_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_borrow_rate(&[(
        &HumanAddr::from("interest"),
        &Decimal::from_ratio(100u128, 1u128),
    )]);

    let borrow_rate = query_borrow_rate(&deps, &HumanAddr::from("interest")).unwrap();

    assert_eq!(
        borrow_rate,
        BorrowRateResponse {
            rate: Decimal::from_ratio(100u128, 1u128),
        }
    );
}

#[test]
fn borrow_limit_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint128::from(1000u128))]);

    let borrow_limit = query_borrow_limit(
        &deps,
        &HumanAddr::from("overseer"),
        &HumanAddr::from("addr0000"),
    )
    .unwrap();

    assert_eq!(
        borrow_limit,
        BorrowLimitResponse {
            borrower: HumanAddr::from("addr0000"),
            borrow_limit: Uint128::from(1000u128),
        }
    );
}

#[test]
fn liquidation_amount_querier() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier
        .with_liquidation_percent(&[(&HumanAddr::from("model0000"), &Decimal::percent(1))]);

    let liquidation_amount = query_liquidation_amount(
        &deps,
        &HumanAddr::from("model0000"),
        Uint128::from(1000000u128),
        Uint128::from(1000000u128),
        "uusd".to_string(),
        vec![
            (HumanAddr::from("token0000"), Uint128::from(1000000u128)),
            (HumanAddr::from("token0001"), Uint128::from(2000000u128)),
            (HumanAddr::from("token0002"), Uint128::from(3000000u128)),
        ],
        Uint128::from(1000000u128),
    )
    .unwrap();
    assert_eq!(
        liquidation_amount,
        LiquidationAmountResponse {
            collaterals: vec![],
        }
    );

    let liquidation_amount = query_liquidation_amount(
        &deps,
        &HumanAddr::from("model0000"),
        Uint128::from(1000001u128),
        Uint128::from(1000000u128),
        "uusd".to_string(),
        vec![
            (HumanAddr::from("token0000"), Uint128::from(1000000u128)),
            (HumanAddr::from("token0001"), Uint128::from(2000000u128)),
            (HumanAddr::from("token0002"), Uint128::from(3000000u128)),
        ],
        Uint128::from(1000000u128),
    )
    .unwrap();
    assert_eq!(
        liquidation_amount,
        LiquidationAmountResponse {
            collaterals: vec![
                (HumanAddr::from("token0000"), Uint128::from(10000u128)),
                (HumanAddr::from("token0001"), Uint128::from(20000u128)),
                (HumanAddr::from("token0002"), Uint128::from(30000u128)),
            ]
        }
    );
}
