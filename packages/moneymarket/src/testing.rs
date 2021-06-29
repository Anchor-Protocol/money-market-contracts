use crate::mock_querier::mock_dependencies;
use crate::oracle::PriceResponse;
use crate::querier::{compute_tax, deduct_tax, query_price, query_tax_rate, TimeConstraints};
use crate::tokens::{Tokens, TokensHuman, TokensMath, TokensToRaw};

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
        Uint256::from(495050u64)
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

#[test]
fn tokens_math() {
    let deps = mock_dependencies(20, &[]);

    let tokens_1: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
        (HumanAddr::from("token3"), Uint256::from(1000000u64)),
        (HumanAddr::from("token5"), Uint256::from(1000000u64)),
    ];

    // not existing item
    let tokens_2: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token4"), Uint256::from(1000000u64)),
    ];

    // not existing item
    let tokens_3: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token6"), Uint256::from(1000000u64)),
    ];

    // sub bigger than source
    let tokens_4: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1200000u64)),
    ];

    let tokens_1_raw: Tokens = tokens_1.to_raw(&deps).unwrap();
    let tokens_2_raw: Tokens = tokens_2.to_raw(&deps).unwrap();
    let tokens_3_raw: Tokens = tokens_3.to_raw(&deps).unwrap();
    let tokens_4_raw: Tokens = tokens_4.to_raw(&deps).unwrap();

    assert_eq!(tokens_1_raw.clone().sub(tokens_2_raw).is_err(), true);
    assert_eq!(tokens_1_raw.clone().sub(tokens_3_raw).is_err(), true);
    assert_eq!(tokens_1_raw.clone().sub(tokens_4_raw).is_err(), true);
}

#[test]
fn tokens_math_normal_add() {
    let deps = mock_dependencies(20, &[]);

    let tokens_1: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
        (HumanAddr::from("token3"), Uint256::from(1000000u64)),
        (HumanAddr::from("token5"), Uint256::from(1000000u64)),
    ];

    let tokens_2: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token4"), Uint256::from(1000000u64)),
    ];

    let mut tokens_1_raw: Tokens = tokens_1.to_raw(&deps).unwrap();
    let tokens_2_raw: Tokens = tokens_2.to_raw(&deps).unwrap();

    tokens_1_raw.add(tokens_2_raw);
    assert_eq!(tokens_1_raw[0].1, Uint256::from(2000000u64));
    assert_eq!(tokens_1_raw.len(), 5);
}

#[test]
fn token_math_zero_token() {
    let deps = mock_dependencies(20, &[]);

    let tokens_1: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
    ];

    let tokens_2: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
    ];

    let mut tokens_1_raw: Tokens = tokens_1.to_raw(&deps).unwrap();
    let tokens_2_raw: Tokens = tokens_2.to_raw(&deps).unwrap();

    tokens_1_raw.sub(tokens_2_raw).unwrap();
    assert_eq!(tokens_1_raw.len(), 0);
}

#[test]
#[should_panic]
fn token_math_invalid_token() {
    let deps = mock_dependencies(20, &[]);

    let tokens_1: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
        (HumanAddr::from("token3"), Uint256::from(1000000u64)),
        (HumanAddr::from("token5"), Uint256::from(1000000u64)),
    ];

    // duplicated item
    let tokens_2: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token3"), Uint256::from(1000000u64)),
    ];

    let tokens_1_raw: Tokens = tokens_1.to_raw(&deps).unwrap();
    let tokens_2_raw: Tokens = tokens_2.to_raw(&deps).unwrap();

    let _ = tokens_1_raw.clone().sub(tokens_2_raw);
}

#[test]
#[should_panic]
fn token_math_invalid_token_2() {
    let deps = mock_dependencies(20, &[]);

    let tokens_1: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
        (HumanAddr::from("token3"), Uint256::from(1000000u64)),
        (HumanAddr::from("token5"), Uint256::from(1000000u64)),
    ];

    // duplicated item
    let tokens_2: TokensHuman = vec![
        (HumanAddr::from("token1"), Uint256::from(1000000u64)),
        (HumanAddr::from("token2"), Uint256::from(1000000u64)),
        (HumanAddr::from("token3"), Uint256::from(1000000u64)),
    ];

    let tokens_1_raw: Tokens = tokens_1.to_raw(&deps).unwrap();
    let tokens_2_raw: Tokens = tokens_2.to_raw(&deps).unwrap();

    let _ = tokens_1_raw.clone().sub(tokens_2_raw);
}
