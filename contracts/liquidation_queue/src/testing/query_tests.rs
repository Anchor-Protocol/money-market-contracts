use crate::contract::{handle, init, query};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{from_binary, Coin, Decimal, HumanAddr, Uint128};
use moneymarket::liquidation_queue::{
    BidPoolResponse, BidPoolsResponse, BidResponse, BidsResponse, HandleMsg, InitMsg,
    LiquidationAmountResponse, QueryMsg,
};

#[test]
fn query_liquidation_amount() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("token0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("token0001"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("token0002"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 5u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100000000000u128),
        }],
    );
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0001"),
        premium_slot: 5u8,
    };
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0002"),
        premium_slot: 5u8,
    };
    handle(&mut deps, env, msg).unwrap();

    // fee_deductor = 0.931095
    // expected_repay_amount = 931,095
    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(931095u64),
        borrow_limit: Uint256::from(900000u64),
        collaterals: vec![(HumanAddr::from("token0000"), Uint256::from(1000000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(&mut deps, msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![(HumanAddr::from("token0000"), Uint256::from(1000000u64))],
        }
    );

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(100000u64),
        borrow_limit: Uint256::from(1000000u64),
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
        borrow_limit: Uint256::from(99999u64),
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

    // fee_deductor = 0.931095
    // liquidation_ratio = 0.3580014213
    let res = query(&mut deps, query_msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                (HumanAddr::from("token0000"), Uint256::from(358001u64)),
                (HumanAddr::from("token0001"), Uint256::from(716002u64)),
                (HumanAddr::from("token0002"), Uint256::from(1074004u64)),
            ],
        }
    );
}

#[test]
fn query_bids() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("token0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 5u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 5u8,
    };
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 10u8,
    };
    handle(&mut deps, env, msg).unwrap();

    let bid_response: BidResponse = from_binary(
        &query(
            &deps,
            QueryMsg::Bid {
                bid_idx: Uint128::from(1u128),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bid_response,
        BidResponse {
            idx: Uint128::from(1u128),
            collateral_token: HumanAddr::from("token0000"),
            bidder: HumanAddr::from("addr0000"),
            amount: Uint256::from(1000u128),
            premium_slot: 5u8,
            pending_liquidated_collateral: Uint256::zero(),
            wait_end: None,
            product_snapshot: Decimal256::one(),
            sum_snapshot: Decimal256::zero(),
            epoch_snapshot: Uint128::zero(),
            scale_snapshot: Uint128::zero(),
        }
    );

    let bids_response: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByUser {
                collateral_token: HumanAddr::from("token0000"),
                bidder: HumanAddr::from("addr0000"),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids_response,
        BidsResponse {
            bids: vec![
                BidResponse {
                    idx: Uint128::from(1u128),
                    collateral_token: HumanAddr::from("token0000"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1000u128),
                    premium_slot: 5u8,
                    pending_liquidated_collateral: Uint256::zero(),
                    wait_end: None,
                    product_snapshot: Decimal256::one(),
                    sum_snapshot: Decimal256::zero(),
                    epoch_snapshot: Uint128::zero(),
                    scale_snapshot: Uint128::zero(),
                },
                BidResponse {
                    idx: Uint128::from(2u128),
                    collateral_token: HumanAddr::from("token0000"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1000u128),
                    premium_slot: 5u8,
                    pending_liquidated_collateral: Uint256::zero(),
                    wait_end: None,
                    product_snapshot: Decimal256::one(),
                    sum_snapshot: Decimal256::zero(),
                    epoch_snapshot: Uint128::zero(),
                    scale_snapshot: Uint128::zero(),
                },
                BidResponse {
                    idx: Uint128::from(3u128),
                    collateral_token: HumanAddr::from("token0000"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1000u128),
                    premium_slot: 10u8,
                    pending_liquidated_collateral: Uint256::zero(),
                    wait_end: None,
                    product_snapshot: Decimal256::one(),
                    sum_snapshot: Decimal256::zero(),
                    epoch_snapshot: Uint128::zero(),
                    scale_snapshot: Uint128::zero(),
                }
            ]
        }
    );

    let bids_response: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByUser {
                collateral_token: HumanAddr::from("token0000"),
                bidder: HumanAddr::from("addr0000"),
                start_after: Some(Uint128(1u128)),
                limit: Some(1u8),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids_response,
        BidsResponse {
            bids: vec![BidResponse {
                idx: Uint128::from(2u128),
                collateral_token: HumanAddr::from("token0000"),
                bidder: HumanAddr::from("addr0000"),
                amount: Uint256::from(1000u128),
                premium_slot: 5u8,
                pending_liquidated_collateral: Uint256::zero(),
                wait_end: None,
                product_snapshot: Decimal256::one(),
                sum_snapshot: Decimal256::zero(),
                epoch_snapshot: Uint128::zero(),
                scale_snapshot: Uint128::zero(),
            }]
        }
    );
}

#[test]
fn query_bid_pools() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("token0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 5u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 6u8,
    };
    handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("token0000"),
        premium_slot: 10u8,
    };
    handle(&mut deps, env, msg).unwrap();

    let bid_pool_response: BidPoolResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidPool {
                collateral_token: HumanAddr::from("token0000"),
                bid_slot: 5u8,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bid_pool_response,
        BidPoolResponse {
            total_bid_amount: Uint256::from(1000u128),
            premium_rate: Decimal256::percent(5),
            sum_snapshot: Decimal256::zero(),
            product_snapshot: Decimal256::one(),
            current_epoch: Uint128::zero(),
            current_scale: Uint128::zero(),
        }
    );

    let bid_pools_response: BidPoolsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidPoolsByCollateral {
                collateral_token: HumanAddr::from("token0000"),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bid_pools_response,
        BidPoolsResponse {
            bid_pools: vec![
                BidPoolResponse {
                    total_bid_amount: Uint256::from(1000u128),
                    premium_rate: Decimal256::percent(5),
                    sum_snapshot: Decimal256::zero(),
                    product_snapshot: Decimal256::one(),
                    current_epoch: Uint128::zero(),
                    current_scale: Uint128::zero(),
                },
                BidPoolResponse {
                    total_bid_amount: Uint256::from(1000u128),
                    premium_rate: Decimal256::percent(6),
                    sum_snapshot: Decimal256::zero(),
                    product_snapshot: Decimal256::one(),
                    current_epoch: Uint128::zero(),
                    current_scale: Uint128::zero(),
                },
                BidPoolResponse {
                    total_bid_amount: Uint256::from(1000u128),
                    premium_rate: Decimal256::percent(10),
                    sum_snapshot: Decimal256::zero(),
                    product_snapshot: Decimal256::one(),
                    current_epoch: Uint128::zero(),
                    current_scale: Uint128::zero(),
                }
            ]
        }
    );

    let bid_pools_response: BidPoolsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidPoolsByCollateral {
                collateral_token: HumanAddr::from("token0000"),
                start_after: Some(1u8),
                limit: Some(1u8),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bid_pools_response,
        BidPoolsResponse {
            bid_pools: vec![BidPoolResponse {
                total_bid_amount: Uint256::from(1000u128),
                premium_rate: Decimal256::percent(5),
                sum_snapshot: Decimal256::zero(),
                product_snapshot: Decimal256::one(),
                current_epoch: Uint128::zero(),
                current_scale: Uint128::zero(),
            },]
        }
    );
}
