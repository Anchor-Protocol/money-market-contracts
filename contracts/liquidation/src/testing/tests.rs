use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use moneymarket::liquidation::{
    BidResponse, BidsResponse, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
    LiquidationAmountResponse, QueryMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        value,
        ConfigResponse {
            owner: "owner0000".to_string(),
            oracle_contract: "oracle0000".to_string(),
            stable_denom: "uusd".to_string(),
            safe_ratio: Decimal256::percent(10),
            bid_fee: Decimal256::percent(1),
            max_premium_rate: Decimal256::percent(5),
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
        }
    );
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        oracle_contract: None,
        stable_denom: None,
        safe_ratio: None,
        bid_fee: None,
        max_premium_rate: None,
        liquidation_threshold: None,
        price_timeframe: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        value,
        ConfigResponse {
            owner: "owner0001".to_string(),
            oracle_contract: "oracle0000".to_string(),
            stable_denom: "uusd".to_string(),
            safe_ratio: Decimal256::percent(10),
            bid_fee: Decimal256::percent(1),
            max_premium_rate: Decimal256::percent(5),
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
        }
    );

    // Update left items
    let info = mock_info("owner0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        oracle_contract: Some("oracle0001".to_string()),
        stable_denom: Some("ukrw".to_string()),
        safe_ratio: Some(Decimal256::percent(15)),
        bid_fee: Some(Decimal256::percent(2)),
        max_premium_rate: Some(Decimal256::percent(7)),
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(120u64),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        value,
        ConfigResponse {
            owner: "owner0001".to_string(),
            oracle_contract: "oracle0001".to_string(),
            stable_denom: "ukrw".to_string(),
            safe_ratio: Decimal256::percent(15),
            bid_fee: Decimal256::percent(2),
            max_premium_rate: Decimal256::percent(7),
            liquidation_threshold: Uint256::from(150000000u64),
            price_timeframe: 120u64,
        }
    );

    // Unauthorized err
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        oracle_contract: Some("oracle0001".to_string()),
        stable_denom: Some("ukrw".to_string()),
        safe_ratio: Some(Decimal256::percent(1)),
        bid_fee: Some(Decimal256::percent(2)),
        max_premium_rate: Some(Decimal256::percent(7)),
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(100u64),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn submit_bid() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(20),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    let _premium_string = "0.05".to_string();
    match res {
        Err(ContractError::PremiumExceedsMaxPremium(_premium_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(1),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    let _uusd = "uusd".to_string();
    match res {
        Err(ContractError::AssetNotProvided(_uusd)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let bid_response: BidResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Bid {
                bidder: "addr0000".to_string(),
                collateral_token: "asset0000".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bid_response,
        BidResponse {
            collateral_token: "asset0000".to_string(),
            bidder: "addr0000".to_string(),
            amount: Uint256::from(1000000u128),
            premium_rate: Decimal256::percent(1),
        }
    );
}

#[test]
fn retract_bid() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(1),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RetractBid {
        collateral_token: "asset0000".to_string(),
        amount: Some(Uint256::from(1000001u64)),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Err(ContractError::RetractExceedsBid(1000000)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::RetractBid {
        collateral_token: "asset0000".to_string(),
        amount: Some(Uint256::from(500000u64)),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500000u128),
            }]
        }))]
    );

    let msg = ExecuteMsg::RetractBid {
        collateral_token: "asset0000".to_string(),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500000u128),
            }]
        }))]
    );
}

#[test]
fn execute_bid() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(50),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(1),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0001".to_string(),
        amount: Uint128::from(2020206u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "addr0000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("asset0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Err(ContractError::InsufficientBidBalance(1000001)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
    // required_stable 495,000
    // bid_fee         4,950
    // repay_amount    490,050
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0001".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "addr0000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(1000000u128),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(485198u128), // 490050 / (1 + tax_rate)
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            })),
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0001".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "addr0000".to_string(),
            fee_address: None,
            repay_address: None,
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(1000000u128),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0001".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(485198u128), // 490050 / (1 + tax_rate)
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0001".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            })),
        ]
    );
}

#[test]
fn query_liquidation_amount() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

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
                ("token0000".to_string(), Uint256::from(358001u64)),
                ("token0001".to_string(), Uint256::from(716002u64)),
                ("token0002".to_string(), Uint256::from(1074004u64)),
            ],
        }
    );
}

#[test]
fn query_bids_by_user() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(50),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(1),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0001".to_string(),
        premium_rate: Decimal256::percent(2),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0002".to_string(),
        premium_rate: Decimal256::percent(3),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(3000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let bids: BidsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByUser {
                bidder: "addr0000".to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids,
        BidsResponse {
            bids: vec![
                BidResponse {
                    collateral_token: "asset0000".to_string(),
                    bidder: "addr0000".to_string(),
                    amount: Uint256::from(1000000u128),
                    premium_rate: Decimal256::percent(1),
                },
                BidResponse {
                    collateral_token: "asset0001".to_string(),
                    bidder: "addr0000".to_string(),
                    amount: Uint256::from(2000000u128),
                    premium_rate: Decimal256::percent(2),
                },
                BidResponse {
                    collateral_token: "asset0002".to_string(),
                    bidder: "addr0000".to_string(),
                    amount: Uint256::from(3000000u128),
                    premium_rate: Decimal256::percent(3),
                }
            ]
        }
    );

    let bids: BidsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByUser {
                bidder: "addr0000".to_string(),
                start_after: Some("asset0000".to_string()),
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids,
        BidsResponse {
            bids: vec![
                BidResponse {
                    collateral_token: "asset0001".to_string(),
                    bidder: "addr0000".to_string(),
                    amount: Uint256::from(2000000u128),
                    premium_rate: Decimal256::percent(2),
                },
                BidResponse {
                    collateral_token: "asset0002".to_string(),
                    bidder: "addr0000".to_string(),
                    amount: Uint256::from(3000000u128),
                    premium_rate: Decimal256::percent(3),
                }
            ]
        }
    );
    let bids: BidsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByUser {
                bidder: "addr0000".to_string(),
                start_after: None,
                limit: Some(1u32),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids,
        BidsResponse {
            bids: vec![BidResponse {
                collateral_token: "asset0000".to_string(),
                bidder: "addr0000".to_string(),
                amount: Uint256::from(1000000u128),
                premium_rate: Decimal256::percent(1),
            }]
        }
    );
}

#[test]
fn query_bids_by_collateral() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(50), 123456u64, 123456u64),
    )]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(1),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_rate: Decimal256::percent(2),
    };
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0001".to_string(),
        premium_rate: Decimal256::percent(3),
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(3000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let bids: BidsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByCollateral {
                collateral_token: "asset0000".to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids,
        BidsResponse {
            bids: vec![
                BidResponse {
                    collateral_token: "asset0000".to_string(),
                    bidder: "addr0000".to_string(),
                    amount: Uint256::from(1000000u128),
                    premium_rate: Decimal256::percent(1),
                },
                BidResponse {
                    collateral_token: "asset0000".to_string(),
                    bidder: "addr0001".to_string(),
                    amount: Uint256::from(2000000u128),
                    premium_rate: Decimal256::percent(2),
                }
            ]
        }
    );

    let bids: BidsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByCollateral {
                collateral_token: "asset0000".to_string(),
                start_after: Some("addr0000".to_string()),
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids,
        BidsResponse {
            bids: vec![BidResponse {
                collateral_token: "asset0000".to_string(),
                bidder: "addr0001".to_string(),
                amount: Uint256::from(2000000u128),
                premium_rate: Decimal256::percent(2),
            }]
        }
    );
    let bids: BidsResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidsByCollateral {
                collateral_token: "asset0000".to_string(),
                start_after: None,
                limit: Some(1u32),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bids,
        BidsResponse {
            bids: vec![BidResponse {
                collateral_token: "asset0000".to_string(),
                bidder: "addr0000".to_string(),
                amount: Uint256::from(1000000u128),
                premium_rate: Decimal256::percent(1),
            }]
        }
    );
}
