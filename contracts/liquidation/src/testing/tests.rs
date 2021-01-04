use crate::contract::{handle, init, query};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128,
    WasmMsg,
};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use moneymarket::liquidation::{
    BidResponse, BidsResponse, ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg,
    LiquidationAmountResponse, QueryMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value: ConfigResponse = from_binary(&query(&deps, QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        value,
        ConfigResponse {
            owner: HumanAddr::from("owner0000"),
            oracle_contract: HumanAddr::from("oracle0000"),
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
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    // update owner
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: Some(HumanAddr("owner0001".to_string())),
        oracle_contract: None,
        stable_denom: None,
        safe_ratio: None,
        bid_fee: None,
        max_premium_rate: None,
        liquidation_threshold: None,
        price_timeframe: None,
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value: ConfigResponse = from_binary(&query(&deps, QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        value,
        ConfigResponse {
            owner: HumanAddr::from("owner0001"),
            oracle_contract: HumanAddr::from("oracle0000"),
            stable_denom: "uusd".to_string(),
            safe_ratio: Decimal256::percent(10),
            bid_fee: Decimal256::percent(1),
            max_premium_rate: Decimal256::percent(5),
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
        }
    );

    // Update left items
    let env = mock_env("owner0001", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        oracle_contract: Some(HumanAddr::from("oracle0001")),
        stable_denom: Some("ukrw".to_string()),
        safe_ratio: Some(Decimal256::percent(15)),
        bid_fee: Some(Decimal256::percent(2)),
        max_premium_rate: Some(Decimal256::percent(7)),
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(120u64),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value: ConfigResponse = from_binary(&query(&deps, QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        value,
        ConfigResponse {
            owner: HumanAddr::from("owner0001"),
            oracle_contract: HumanAddr::from("oracle0001"),
            stable_denom: "ukrw".to_string(),
            safe_ratio: Decimal256::percent(15),
            bid_fee: Decimal256::percent(2),
            max_premium_rate: Decimal256::percent(7),
            liquidation_threshold: Uint256::from(150000000u64),
            price_timeframe: 120u64,
        }
    );

    // Unauthorized err
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        oracle_contract: Some(HumanAddr::from("oracle0001")),
        stable_denom: Some("ukrw".to_string()),
        safe_ratio: Some(Decimal256::percent(1)),
        bid_fee: Some(Decimal256::percent(2)),
        max_premium_rate: Some(Decimal256::percent(7)),
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(100u64),
    };

    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn submit_bid() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(20),
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "premium_rate cannot be bigger than max_premium_rate")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(1),
    };
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Must provide stable_denom asset"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    let bid_response: BidResponse = from_binary(
        &query(
            &deps,
            QueryMsg::Bid {
                bidder: HumanAddr::from("addr0000"),
                collateral_token: HumanAddr::from("asset0000"),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bid_response,
        BidResponse {
            collateral_token: HumanAddr::from("asset0000"),
            bidder: HumanAddr::from("addr0000"),
            amount: Uint256::from(1000000u128),
            premium_rate: Decimal256::percent(1),
        }
    );
}

#[test]
fn retract_bid() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(1),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::RetractBid {
        collateral_token: HumanAddr::from("asset0000"),
        amount: Some(Uint256::from(1000001u64)),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Cannot retract bigger amount than the bid balance")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::RetractBid {
        collateral_token: HumanAddr::from("asset0000"),
        amount: Some(Uint256::from(500000u64)),
    };
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: HumanAddr::from("addr0000"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500000u128),
            }]
        })]
    );

    let msg = HandleMsg::RetractBid {
        collateral_token: HumanAddr::from("asset0000"),
        amount: None,
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: HumanAddr::from("addr0000"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(500000u128),
            }]
        })]
    );
}

#[test]
fn execute_bid() {
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
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(50), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(1),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(2020206u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("addr0000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    let env = mock_env("asset0000", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Bid amount is smaller than required_stable")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
    // required_stable 495,000
    // bid_fee         4,950
    // repay_amount    490,050
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(1000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("addr0000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: HumanAddr::from("addr0000"),
                    amount: Uint128::from(1000000u128),
                })
                .unwrap(),
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("repay0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(485199u128), // 490050 / (1 + tax_rate)
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("fee0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4901u128), // 4950 / (1 + tax_rate)
                }]
            }),
        ]
    );

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(1000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("addr0000"),
                fee_address: None,
                repay_address: None,
            })
            .unwrap(),
        ),
    });
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: HumanAddr::from("addr0000"),
                    amount: Uint128::from(1000000u128),
                })
                .unwrap(),
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("addr0001"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(485199u128), // 490050 / (1 + tax_rate)
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("addr0001"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4901u128), // 4950 / (1 + tax_rate)
                }]
            }),
        ]
    );
}

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
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    // fee_deductor = 0.931095
    // expected_repay_amount = 931,095
    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(931095u64),
        borrow_limit: Uint256::from(1000000u64),
        collaterals: vec![(HumanAddr::from("token0000"), Uint256::from(1000000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(&mut deps, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "cannot liquidate a undercollateralized loan")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

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
        borrow_limit: Uint256::from(1000000u64),
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
    let res = query(&mut deps, query_msg.clone()).unwrap();
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
fn query_bids_by_user() {
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
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(50), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(1),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0001"),
        premium_rate: Decimal256::percent(2),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0002"),
        premium_rate: Decimal256::percent(3),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(3000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let bids: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByUser {
                bidder: HumanAddr::from("addr0000"),
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
                    collateral_token: HumanAddr::from("asset0000"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1000000u128),
                    premium_rate: Decimal256::percent(1),
                },
                BidResponse {
                    collateral_token: HumanAddr::from("asset0001"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(2000000u128),
                    premium_rate: Decimal256::percent(2),
                },
                BidResponse {
                    collateral_token: HumanAddr::from("asset0002"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(3000000u128),
                    premium_rate: Decimal256::percent(3),
                }
            ]
        }
    );

    let bids: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByUser {
                bidder: HumanAddr::from("addr0000"),
                start_after: Some(HumanAddr::from("asset0000")),
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
                    collateral_token: HumanAddr::from("asset0001"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(2000000u128),
                    premium_rate: Decimal256::percent(2),
                },
                BidResponse {
                    collateral_token: HumanAddr::from("asset0002"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(3000000u128),
                    premium_rate: Decimal256::percent(3),
                }
            ]
        }
    );
    let bids: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByUser {
                bidder: HumanAddr::from("addr0000"),
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
                collateral_token: HumanAddr::from("asset0000"),
                bidder: HumanAddr::from("addr0000"),
                amount: Uint256::from(1000000u128),
                premium_rate: Decimal256::percent(1),
            }]
        }
    );
}

#[test]
fn query_bids_by_collateral() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(50), 123456u64, 123456u64),
    )]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        max_premium_rate: Decimal256::percent(5),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(1),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_rate: Decimal256::percent(2),
    };
    let env = mock_env(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0001"),
        premium_rate: Decimal256::percent(3),
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(3000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let bids: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByCollateral {
                collateral_token: HumanAddr::from("asset0000"),
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
                    collateral_token: HumanAddr::from("asset0000"),
                    bidder: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1000000u128),
                    premium_rate: Decimal256::percent(1),
                },
                BidResponse {
                    collateral_token: HumanAddr::from("asset0000"),
                    bidder: HumanAddr::from("addr0001"),
                    amount: Uint256::from(2000000u128),
                    premium_rate: Decimal256::percent(2),
                }
            ]
        }
    );

    let bids: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByCollateral {
                collateral_token: HumanAddr::from("asset0000"),
                start_after: Some(HumanAddr::from("addr0000")),
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
                collateral_token: HumanAddr::from("asset0000"),
                bidder: HumanAddr::from("addr0001"),
                amount: Uint256::from(2000000u128),
                premium_rate: Decimal256::percent(2),
            }]
        }
    );
    let bids: BidsResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidsByCollateral {
                collateral_token: HumanAddr::from("asset0000"),
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
                collateral_token: HumanAddr::from("asset0000"),
                bidder: HumanAddr::from("addr0000"),
                amount: Uint256::from(1000000u128),
                premium_rate: Decimal256::percent(1),
            }]
        }
    );
}
