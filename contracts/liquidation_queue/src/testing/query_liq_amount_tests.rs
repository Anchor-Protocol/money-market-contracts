use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, SubMsg, Uint128};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, LiquidationAmountResponse, QueryMsg,
};

#[test]
fn partial_one_collateral_one_slot_high_ltv() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(90))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
        premium_slot: 0u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(19000u64),
        borrow_limit: Uint256::from(18000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))], // value 20000 (LTV 90%), limit = 18,000
        collateral_prices: vec![Decimal256::percent(100)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(16433u64))],
        }
    );

    // 16433 col liq
    // remaining = 20000 - 16433 = 3,567
    // new limit = 3,567 * 1 * 0.9 = 3,210
    // safe = 3,210 * 0.8 = 2,568 **

    // new borrow amount = 19000 - 16433 = 2,567 **

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(100),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(16433u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(16433u128),
            }]
        }))]
    );
}

#[test]
fn partial_one_collateral_one_slot() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(7291u64))],
        }
    );

    // 7291 col liq
    // remaining = 20000 - 7291 = 12709
    // new limit = 12709 * 0.1 * 0.5 = 635
    // safe = 635 * 0.8 = 508

    // new borrow amount = 1200 - 692 = 508

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(7291u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(692u128),
            }]
        }))]
    );
}

#[test]
fn partial_one_collateral_one_slot_with_fees() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(7551u64))],
        }
    );

    // 7551 col liq
    // remaining = 20000 - 7551 = 12,449
    // new limit = 12,449 * 0.1 * 0.5 = 622
    // safe = 622 * 0.8 = 497

    // new borrow amount = 1200 - 702 = 498

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(7551u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(702u128),
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(6u128),
                }]
            }))
        ]
    );
}

#[test]
fn partial_one_collateral_one_slot_with_fees_all() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(1),
        liquidation_threshold: Uint256::zero(),
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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(7686u64))],
        }
    );

    // 7686 col liq
    // remaining = 20000 - 7686 = 12,314
    // new limit = 12,314 * 0.1 * 0.5 = 615.7
    // safe = 615.7 * 0.8 = 492.56

    // new borrow amount = 1200 - 708 = 492

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(7686u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(708u128),
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(6u128),
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "liquidator00000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(6u128),
                }]
            }))
        ]
    );
}

#[test]
fn partial_one_collateral_two_slots() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000000u128), // to get instant activation
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
            amount: Uint128::from(3000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(10300u64),
        borrow_limit: Uint256::from(10000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(200000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(42861u64))],
        }
    );

    // 42861 col liq
    // remaining = 200000 - 42861 = 157,139
    // new limit = 157140 * 0.1 * 0.5 = 7,856
    // safe = 7,856 * 0.8 = 6,284 ****

    // repay amount = 4015

    // new borrow amount = 10300 - 4015 = 6,285 ****

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(42860u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(4015u128), // repay amount = 4015
            }]
        }))]
    );
}

#[test]
fn partial_one_collateral_two_slots_with_fees() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000000u128), // to get instant activation
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
            amount: Uint128::from(3000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(10300u64),
        borrow_limit: Uint256::from(10000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(200000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(44453u64))],
        }
    );

    // 44453 col liq
    // remaining = 200000 - 44453 = 155,547
    // new limit = 155,547 * 0.1 * 0.5 = 7,777
    // safe = 7,777 * 0.8 = 6,221 ****

    // repay amount = 4076

    // new borrow amount = 10300 - 4076 = 6,224 ****

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(44453u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4076u128), // repay amount = 4076
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(40u128),
                }]
            }))
        ]
    );
}

#[test]
fn non_partial_liquidation() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(1000000u128),
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
            amount: Uint128::from(3000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(12643u64))],
        }
    );

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(12643u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1201u128), // repay all borrowed amount, overpay
            }]
        }))]
    );
}

#[test]
fn non_partial_liquidation_two_slots() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(1000000u128),
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
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(12756u64))],
        }
    );

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(12756u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1200u128), // repay all borrowed amount
            }]
        }))]
    );
}

#[test]
fn non_partial_liquidation_with_fees() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(1000000u128),
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
            amount: Uint128::from(3000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(12899u64))],
        }
    );

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(12899u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1200u128), // repay all borrowed amount
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(11u128),
                }]
            }))
        ]
    );
}

#[test]
fn non_partial_liquidation_two_slots_with_fees() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(1000000u128),
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
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200u64),
        borrow_limit: Uint256::from(1000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000u64))],
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(13015u64))],
        }
    );

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(13015u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1200u128), // repay all borrowed amount
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(11u128),
                }]
            }))
        ]
    );
}

#[test]
fn non_partial_liquidation_two_slots_with_fees_big_nums() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"token0000".to_string(), &Decimal256::percent(50))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(2000000000u128),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000000000000u128), // to get instant activation
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
            amount: Uint128::from(1500000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1300000000u64),
        borrow_limit: Uint256::from(1000000000u64),
        collaterals: vec![("token0000".to_string(), Uint256::from(20000000000u64))], // value = 2,000,000,000
        collateral_prices: vec![Decimal256::percent(10)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(13833067518u64))],
        }
    );

    let info = mock_info("token0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(10),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(13833067518u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1300000000u128), // exact loan amount
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(13011300u128),
                }]
            }))
        ]
    );
}

#[test]
fn partial_two_collaterals_ltv_diff() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(99)),
        (&"token0001".to_string(), &Decimal256::percent(1)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000000000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 0u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200000000u64),
        borrow_limit: Uint256::from(1000000000u64),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(1000000000u64)), // value = 1000000000 (LTV 99%) limit = 990..
            ("token0001".to_string(), Uint256::from(1000000000u64)), // value = 1000000000 (LTV 1%) limit = 10..
        ],
        collateral_prices: vec![Decimal256::percent(100), Decimal256::percent(100)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(19230775u64)),
                ("token0001".to_string(), Uint256::from(399193550u64))
            ],
        }
    );

    // 19230775 col1 liq
    // remaining = 1000000000 - 19230775 = 980,769,225

    // 399193550 col2 liq
    // remaining = 1000000000 - 399193550 = 600,806,450

    // new limit = (980,769,225 * 1) * 0.99 + (600,806,450 * 1) * 0.01 = 976,969,597
    // safe = 976,969,597 * 0.8 = 781,575,677 **

    // repayed = 19230775 +  399193550 = 418,424,325
    // new borrow amount = 1200000000 - 418,424,325 = 781,575,675 **

    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(100),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(100),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(19230775u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(19230775u128),
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(399193550u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(399193550u128),
            }]
        }))]
    );
}

#[test]
fn partial_two_collaterals_multi_slots_per_col() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(280u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 11u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 3u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1700u64),
        borrow_limit: Uint256::from(1450u64),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(20000u64)), // value = 2000 (LTV = 50%) limit = 1000
            ("token0001".to_string(), Uint256::from(30000u64)), // value = 1500 (LTV = 30%) limit = 450
        ],
        collateral_prices: vec![Decimal256::percent(10), Decimal256::percent(5)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(3796u64)),
                ("token0001".to_string(), Uint256::from(9637u64))
            ],
        }
    );

    // 3796 col1 liq
    // remaining = 20000 - 3796 = 16,204

    // 9637 col2 liq
    // remaining = 30000 - 9637 = 20,363

    // new limit = (16,204 * 0.1) * 0.5 + (20,363 * 0.05) * 0.3 = 1,115
    // safe = 1,115 * 0.8 = 892 **

    // repayed = 355 + 453 = 808
    // new borrow amount = 1700 - 808 = 892 **

    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(10),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(5),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(3796u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(355u128),
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(9637u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(453u128),
            }]
        }))]
    );
}

#[test]
fn partial_two_collaterals_one_slot_diff_ltv() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
        collateral_token: "token0001".to_string(),
        premium_slot: 5u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1600u64),
        borrow_limit: Uint256::from(1450u64),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(20000u64)), // value = 2000 LTV = 50% (limit = 1000)
            ("token0001".to_string(), Uint256::from(30000u64)), // value = 1500 LTV = 30% (limit = 450)
        ],
        collateral_prices: vec![Decimal256::percent(10), Decimal256::percent(5)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(3037u64)),
                ("token0001".to_string(), Uint256::from(7775u64))
            ],
        }
    );

    // 3037 col1 liq
    // remaining = 20000 - 3037 = 16,963

    // 7775 col2 liq
    // remaining = 30000 - 7775 = 22,225

    // new limit = (16,963 * 0.1) * 0.5 + (22,225 * 0.05) * 0.3 = 1,181
    // safe = 1,181 * 0.8 = 944 **

    // repayed = 288 + 369 = 657
    // new borrow amount = 1600 - 657 = 943 **

    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(10),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(5),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(3037u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(288u128), // col1 repay = 288
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(7775u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(369u128), // col2 repay = 369
            }]
        }))]
    );
}

#[test]
fn partial_three_collaterals_one_slot_diff_ltv() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(70)),
        (&"token0002".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
        bid_threshold: Uint256::from(1000000u128), // to get instant activation
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
            amount: Uint128::from(100000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0002".to_string(),
        premium_slot: 1u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(4500u64),
        borrow_limit: Uint256::from(4030u64),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(20000u64)), // value = 2000 LTV = 50% (limit = 1000)
            ("token0001".to_string(), Uint256::from(30000u64)), // value = 1500 LTV = 70% (limit = 1,050)
            ("token0002".to_string(), Uint256::from(6000u64)), // value = 6600 LTV = 30% (limit = 1,980)
        ],
        collateral_prices: vec![
            Decimal256::percent(10),
            Decimal256::percent(5),
            Decimal256::percent(110),
        ],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(3328u64)),
                ("token0001".to_string(), Uint256::from(5824u64)),
                ("token0002".to_string(), Uint256::from(1210u64)),
            ],
        }
    );

    // 3328 col1 liq
    // remaining = 20000 - 3328 = 16,672

    // 5824 col2 liq
    // remaining = 30000 - 5824 = 24,176

    // 1210 col3 liq
    // remaining = 6000 - 1210 = 4,790

    // new limit = (16,672 * 0.1) * 0.5 + (24,176 * 0.05) * 0.7 + (4,790 * 1.1) * 0.3 = 3,260
    // safe = 3,260 * 0.8 = 2,608 **

    // repayed = 316 + 262 + 1317 = 1,895
    // new borrow amount = 4500 - 1,895 = 2,605 **
    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(10),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(5),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0002".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(110),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(3328u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(316u128), // col1 repay = 316
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(5824u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(262u128), // col2 repay = 262
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(1210u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0002", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1317u128), // col3 repay = 1317
            }]
        }))]
    );
}

#[test]
fn partial_three_collaterals_one_slot_diff_ltv_big_amounts() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(40)),
        (&"token0002".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0001".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0002".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000u128), // to get instant activation
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
            amount: Uint128::from(1000000000000u128), // 1M UST
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0002".to_string(),
        premium_slot: 1u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Bids
    // 1M @ 1% col1
    // 1M @ 5% col2
    // 1M @ 10% col3

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(1200000000000u64),
        borrow_limit: Uint256::from(1007980000000u128),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(200000000000u64)), // value = 2,000,000,000,000 LTV = 50% (limit = 1,000,000,000,000)
            ("token0001".to_string(), Uint256::from(3000000000u64)), // value = 15,000,000,000 LTV = 40% (limit = 6,000,000,000)
            ("token0002".to_string(), Uint256::from(60000000u64)), // value = 6,600,000,000 LTV = 30% (limit = 1,980,000,000)
        ],
        collateral_prices: vec![
            Decimal256::percent(1000),
            Decimal256::percent(500),
            Decimal256::percent(11000),
        ],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(69498951644u64)),
                ("token0001".to_string(), Uint256::from(2471406687u64)),
                ("token0002".to_string(), Uint256::from(50965898u64)),
            ],
        }
    );

    // 69498951644 col1 liq
    // remaining = 200000000000 - 69498951644 = 130,501,048,356

    // 2471406687 col2 liq
    // remaining = 3000000000 - 2471406687 = 528,593,313

    // 50965898 col3 liq
    // remaining = 60000000 - 50965898 = 9,034,102

    // new limit = (130,501,048,356 * 10) * 0.5 + (528,593,313 * 5) * 0.4 + (9,034,102 * 110) * 0.3 = 653,860,553,772
    // safe = 653,860,553,772 * 0.8 = 523,088,443,017 **

    // repayed = 660240040618 + 11121330091 + 5550186292 = 676,911,557,001
    // new borrow amount = 1200000000000 - 676,911,557,001= 523,088,442,999 **

    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(1000),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(500),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0002".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(11000),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(69498951644u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(660240040618u128), // col1 repay = 660240040618
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(2471406687u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(11121330091u128), // col2 repay = 11121330091
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(50965898u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0002", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(5550186292u128), // col3 repay = 5550186292
            }]
        }))]
    );
}

#[test]
fn partial_three_collaterals_one_slot_diff_ltv_big_amounts_2() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(0),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(40)),
        (&"token0002".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0001".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0002".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000u128), // to get instant activation
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
            amount: Uint128::from(10000000000000000000u128), // 1M UST
        }],
    );
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0002".to_string(),
        premium_slot: 1u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Bids
    // 1M @ 1% col1
    // 1M @ 5% col2
    // 1M @ 10% col3

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(9100000000u64),
        borrow_limit: Uint256::from(8980000000u128),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(200000000u64)), // value = 2,000,000,000 LTV = 50% (limit = 1,000,000,000)
            ("token0001".to_string(), Uint256::from(3000000000u64)), // value = 15,000,000,000 LTV = 40% (limit = 6,000,000,000)
            ("token0002".to_string(), Uint256::from(60000000u64)), // value = 6,600,000,000 LTV = 30% (limit = 1,980,000,000)
        ],
        collateral_prices: vec![
            Decimal256::percent(1000),
            Decimal256::percent(500),
            Decimal256::percent(11000),
        ],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(21944167u64)),
                ("token0001".to_string(), Uint256::from(390171057u64)),
                ("token0002".to_string(), Uint256::from(8046195u64)),
            ],
        }
    );

    // 21944167 col1 liq
    // remaining = 200000000 - 21944167 = 178,055,833

    // 390171057 col2 liq
    // remaining = 3000000000 - 390171057 = 2,609,828,943

    // 8046195 col3 liq
    // remaining = 60000000 - 8046195 = 51,953,805

    // new limit = (178,055,833 * 10) * 0.5 + (2,609,828,943 * 5) * 0.4 + (51,953,805 * 110) * 0.3 = 7,824,412,616
    // safe = 7,824,412,616 * 0.8 = 6,259,530,092 **

    // repayed = 219350041 + 1755769756 + 876230635 = 2,851,350,432
    // new borrow amount = 9100000000 - 2,851,350,432 = 6,248,649,568 **

    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(1000),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(500),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0002".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(11000),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(23089478u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(219350041u128), // col1 repay = 219350041
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(390171057u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1755769756u128), // col2 repay = 1755769756
            }]
        }))]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(8046195u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0002", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(876230635u128), // col3 repay = 876230635
            }]
        }))]
    );
}

#[test]
fn not_enough_bids_for_one_of_two_col() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(50)),
        (&"token0001".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(400000u128), // not enough bids on token0001
        }],
    );
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0001".to_string(),
        premium_slot: 10u8,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(2800000u64),
        borrow_limit: Uint256::from(2450000u64),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(20000000u64)), // value = 2,000,000 (LTV 50%) limit = 2,000,000
            ("token0001".to_string(), Uint256::from(30000000u64)), // value = 1,500,000 (LTV 30%) limit = 450,000
        ],
        collateral_prices: vec![Decimal256::percent(10), Decimal256::percent(5)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![
                ("token0000".to_string(), Uint256::from(11862304u64)),
                ("token0001".to_string(), Uint256::from(6541171u64))
            ],
        }
    );

    // 11862304 col1 liq
    // remaining = 20000000 - 11862304 = 8,137,696

    // 6541171 col2 liq
    // remaining = 30000000 - 6541171 = 23,458,829

    // new limit = (8,137,696 * 0.1) * 0.5 + (23,458,829 * 0.05) * 0.3 = 758,767
    // safe = 758,767 * 0.8 = 607,013 **

    // repayed = 1104602 + 288523 = 1,393,125
    // new borrow amount = 2800000 - 1,393,125 = 1,406,875 // loan cant be brought back to safe

    let env = mock_env();
    deps.querier.with_oracle_price(&[
        (
            &("token0000".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(10),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("token0001".to_string(), "uusd".to_string()),
            &(
                Decimal256::percent(5),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(11862304u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1104602u128),
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(11157u128),
                }]
            }))
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(6541171u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(288523u128),
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(2913u128),
                }]
            }))
        ]
    );
}

#[test]
fn integration_test_simul() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_collateral_max_ltv(&[
        (&"token0000".to_string(), &Decimal256::percent(60)),
        (&"token0001".to_string(), &Decimal256::percent(30)),
    ]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(80),
        bid_fee: Decimal256::percent(0),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::zero(),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "token0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(100000000000000000000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    for slot in 1..30 {
        let msg = ExecuteMsg::SubmitBid {
            collateral_token: "token0000".to_string(),
            premium_slot: slot as u8,
        };
        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(300u128),
            }],
        );
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "token0000".to_string(),
        premium_slot: 30u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(3000000000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::LiquidationAmount {
        borrow_amount: Uint256::from(6000000000u64),
        borrow_limit: Uint256::from(5400000000u64),
        collaterals: vec![
            ("token0000".to_string(), Uint256::from(10000000000u64)), // value = 9,000,000,000 (LTV 60%) limit = 5,400,000,000
        ],
        collateral_prices: vec![Decimal256::percent(90)],
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: LiquidationAmountResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        LiquidationAmountResponse {
            collaterals: vec![("token0000".to_string(), Uint256::from(8489891541u64)),],
        }
    );

    // 8489891541

    // 10000000000 - 8489891541 = 1,510,108,459
    // 1,510,108,459 * 0.9 * 0.6 * 0.8 = 652,366,854

    // 6000000000 - 5347633145 = 652,366,855

    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("token0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(90),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(8489891541u64),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("token0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "repay0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(5347633145u128),
            }]
        }))]
    );
}
