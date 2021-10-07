use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, Coin, Decimal, Uint128};
use moneymarket::liquidation_queue::{
    BidPoolResponse, BidPoolsResponse, BidResponse, BidsResponse, CollateralInfoResponse,
    ExecuteMsg, InstantiateMsg, LiquidationAmountResponse, QueryMsg,
};

#[test]
fn query_liquidation_amount() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(50)),
        (&"token0002".to_string(), &Decimal256::percent(50)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0001".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0002".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100000000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 5u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0002".to_string(),
        premium_slot: 5u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // fee_deductor = 0.931095
    // expected_repay_amount = 931,095
    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(931095u64),
        borrow_limit: Uint256::from(900000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(1000000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(1000000u64))],
        }
    );

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(100000u64),
        borrow_limit: Uint256::from(1000000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(1000000u64))],
        collateral_prices: vec![Decimal256::one()],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
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
            ("token0000".to_string(), Uint256::from(1000000u64)),
            ("token0001".to_string(), Uint256::from(2000000u64)),
            ("token0002".to_string(), Uint256::from(3000000u64)),
        ],
        collateral_prices: vec![
            Decimal256::percent(50),
            Decimal256::percent(50),
            Decimal256::percent(50),
        ],
    };

    // fee_deductor = 0.931095
    // liquidation_ratio = 0.3580014213
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(358003u64)),
                ("token0001".to_string(), Uint256::from(716005u64)),
                ("token0002".to_string(), Uint256::from(1074007u64)),
            ],
        }
    );
}

#[test]
fn query_bids() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(90))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 5u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let bid_response: BidResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
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
            collateral_token: "token0000".to_string(),
            bidder: "addr0000".to_string(),
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
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByUser {
                collateral_token: "token0000".to_string(),
                bidder: "addr0000".to_string(),
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
                    collateral_token: "token0000".to_string(),
                    bidder: "addr0000".to_string(),
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
                    collateral_token: "token0000".to_string(),
                    bidder: "addr0000".to_string(),
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
                    collateral_token: "token0000".to_string(),
                    bidder: "addr0000".to_string(),
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
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByUser {
                collateral_token: "token0000".to_string(),
                bidder: "addr0000".to_string(),
                start_after: Some(Uint128::from(1u128)),
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
                collateral_token: "token0000".to_string(),
                bidder: "addr0000".to_string(),
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
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(90))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 6u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let bid_pool_response: BidPoolResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidPool {
                collateral_token: "token0000".to_string(),
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
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidPoolsByCollateral {
                collateral_token: "token0000".to_string(),
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
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidPoolsByCollateral {
                collateral_token: "token0000".to_string(),
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

#[test]
fn query_collateral_info() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(90))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let collateral_info_response: CollateralInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::CollateralInfo {
                collateral_token: "token0000".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        collateral_info_response,
        CollateralInfoResponse {
            collateral_token: "token0000".to_string(),
            max_slot: 30u8,
            bid_threshold: Uint256::from(10000u128),
            premium_rate_per_slot: Decimal256::percent(1),
        }
    );
}
