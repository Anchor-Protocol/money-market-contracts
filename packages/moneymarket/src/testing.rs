use crate::mock_querier::mock_dependencies;
use crate::oracle::PriceResponse;
use crate::querier::{compute_tax, deduct_tax, query_price, query_tax_rate, TimeConstraints};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Coin, Decimal, HumanAddr, StdError, Uint128};

#[test]
fn tax_rate_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_tax(Decimal::percent(1), &[]);
    assert_eq!(query_tax_rate(&deps).unwrap(), Decimal256::percent(1),);
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
        Uint256::from(1000000u64)
    );

    // normal tax
    assert_eq!(
        compute_tax(&deps, &Coin::new(50000000u128, "uusd")).unwrap(),
        Uint256::from(495049u64)
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
            amount: Uint128(49504951u128)
        }
    );
}

#[test]
fn oracle_price_querier() {
    let mut deps = mock_dependencies(20, &[]);

    deps.querier.with_oracle_price(&[(
        &("terra123123".to_string(), "uusd".to_string()),
        &(Decimal256::from_ratio(131, 2), 123, 321),
    )]);

    let oracle_price = query_price(
        &deps,
        &HumanAddr::from("oracle"),
        "terra123123".to_string(),
        "uusd".to_string(),
        None,
    )
    .unwrap();

    assert_eq!(
        oracle_price,
        PriceResponse {
            rate: Decimal256::from_ratio(131, 2),
            last_updated_base: 123,
            last_updated_quote: 321,
        }
    );

    query_price(
        &deps,
        &HumanAddr::from("oracle"),
        "terra123123".to_string(),
        "ukrw".to_string(),
        None,
    )
    .unwrap_err();

    let res = query_price(
        &deps,
        &HumanAddr::from("oracle"),
        "terra123123".to_string(),
        "uusd".to_string(),
        Some(TimeConstraints {
            block_time: 500u64,
            valid_timeframe: 60u64,
        }),
    );

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Price is too old"),
        _ => panic!("DO NOT ENTER HERE"),
    }
}
