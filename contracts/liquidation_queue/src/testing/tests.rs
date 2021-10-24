use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    attr, from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, StdError, SubMsg, Uint128,
};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    BidResponse, CollateralInfoResponse, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
    QueryMsg,
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
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
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
            liquidator_fee: Decimal256::percent(0),
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
            waiting_period: 60u64,
            overseer: "overseer0000".to_string(),
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
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        oracle_contract: None,
        safe_ratio: None,
        bid_fee: None,
        liquidator_fee: None,
        liquidation_threshold: None,
        price_timeframe: None,
        waiting_period: None,
        overseer: None,
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
            liquidator_fee: Decimal256::percent(0),
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
            waiting_period: 60u64,
            overseer: "overseer0000".to_string(),
        }
    );

    // Update left items
    let info = mock_info("owner0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        oracle_contract: Some("oracle0001".to_string()),
        safe_ratio: Some(Decimal256::percent(15)),
        bid_fee: Some(Decimal256::percent(2)),
        liquidator_fee: Some(Decimal256::percent(1)),
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(120u64),
        waiting_period: Some(100u64),
        overseer: Some("overseer0001".to_string()),
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
            stable_denom: "uusd".to_string(),
            safe_ratio: Decimal256::percent(15),
            bid_fee: Decimal256::percent(2),
            liquidator_fee: Decimal256::percent(1),
            liquidation_threshold: Uint256::from(150000000u64),
            price_timeframe: 120u64,
            waiting_period: 100u64,
            overseer: "overseer0001".to_string(),
        }
    );

    // Unauthorized err
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        oracle_contract: Some("oracle0001".to_string()),
        safe_ratio: Some(Decimal256::percent(1)),
        bid_fee: Some(Decimal256::percent(2)),
        liquidator_fee: Some(Decimal256::percent(1)),
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(100u64),
        waiting_period: Some(100u64),
        overseer: Some("overseer0001".to_string()),
    };

    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));
}

#[test]
fn submit_bid() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_collateral_max_ltv(&[(&"asset0000".to_string(), &Decimal256::percent(90))]);

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
        collateral_token: "asset0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("No uusd assets have been provided")
    );

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid asset provided, only uusd allowed")
    );

    let info = mock_info(
        "addr0000",
        &[
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000u128),
            },
            Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(1000000u128),
            },
        ],
    );
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Invalid asset provided, only uusd allowed")
    );

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

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
            collateral_token: "asset0000".to_string(),
            bidder: "addr0000".to_string(),
            amount: Uint256::from(1000000u128),
            premium_slot: 1u8,
            product_snapshot: Decimal256::one(),
            sum_snapshot: Decimal256::zero(),
            pending_liquidated_collateral: Uint256::zero(),
            wait_end: Some(wait_end.seconds()),
            epoch_snapshot: Uint128::zero(),
            scale_snapshot: Uint128::zero(),
        }
    );
}

#[test]
fn activate_bid() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_collateral_max_ltv(&[(&"asset0000".to_string(), &Decimal256::percent(90))]);

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
        collateral_token: "asset0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "asset0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u64)]),
    };
    let info = mock_info("addr0001", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    let err = execute(deps.as_mut(), env, info, msg.clone()).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    let info = mock_info("addr0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end.minus_seconds(2u64);
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(format!("Wait period expires at {}", wait_end.seconds()))
    );

    // graceful return when idx is not specified
    let msg2 = ExecuteMsg::ActivateBids {
        collateral_token: "asset0000".to_string(),
        bids_idx: None,
    };
    let res = execute(deps.as_mut(), env, info, msg2).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "activate_bids"), attr("amount", "0"),]
    );

    let info = mock_info("addr0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "activate_bids"), attr("amount", "1000000"),]
    );
    assert!(res.messages.is_empty());

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
            collateral_token: "asset0000".to_string(),
            bidder: "addr0000".to_string(),
            amount: Uint256::from(1000000u128),
            premium_slot: 1u8,
            product_snapshot: Decimal256::one(),
            sum_snapshot: Decimal256::zero(),
            pending_liquidated_collateral: Uint256::zero(),
            wait_end: None,
            epoch_snapshot: Uint128::zero(),
            scale_snapshot: Uint128::zero(),
        }
    );
}

#[test]
fn retract_bid() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_collateral_max_ltv(&[(&"asset0000".to_string(), &Decimal256::percent(90))]);

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
        collateral_token: "asset0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "asset0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u64)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    let info = mock_info("addr0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000u128),
            }]
        }))]
    );
}

#[test]
fn retract_unactive_bid() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_collateral_max_ltv(&[(&"asset0000".to_string(), &Decimal256::percent(90))]);

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
        collateral_token: "asset0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(500000u128)),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "500000"),
        ]
    );

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "asset0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("addr0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "activate_bids"), attr("amount", "500000"),]
    );
}

#[test]
fn execute_bid() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"asset0000".to_string(), &Decimal256::percent(90))]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(1),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 100000u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
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

    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "asset0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "asset0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    let info = mock_info("addr0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // required_stable 495,000
    // bid_fee         4,950
    // liquidator_fee  4,950
    // repay_amount    485,100
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0001".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });

    // unauthorized attempt
    let info = mock_info("asset0000", &[]);
    let env = mock_env();
    let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Unauthorized: only custody contract can execute liquidations",)
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(), // only custody contract can execute
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator0000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "repay0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(480297u128), // 485100 / (1 + tax_rate)
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fee0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "liquidator0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            })),
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator0000".to_string(),
            fee_address: None,
            repay_address: None,
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "custody0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(480297u128), // 485100 / (1 + tax_rate)
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "custody0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "liquidator0000".to_string(),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            })),
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(2020206u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("Not enough bids to execute this liquidation")
    );
}

#[test]
fn claim_liquidations() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"asset0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 1000000u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
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

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "asset0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "asset0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "asset0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // required_stable 495,000
    // bid_fee         4,950
    // repay_amount    490,050
    let info = mock_info("asset0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "asset0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "asset0000"),
            attr("collateral_amount", "1000000"),
        ]
    );
}

#[test]
fn update_collateral_info() {
    let mut deps = mock_dependencies(&[]);
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

    let msg = ExecuteMsg::UpdateCollateralInfo {
        collateral_token: "token0000".to_string(),
        bid_threshold: Some(Uint256::from(20000u128)),
        max_slot: Some(20u8),
    };

    // unauthorized attempt
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    // successfull attempt
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query col info
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
            max_slot: 20u8,                          // updated max_slot
            bid_threshold: Uint256::from(20000u128), // updated bid threshold
            premium_rate_per_slot: Decimal256::percent(1),
        }
    );
}
