use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, to_binary, Coin, Decimal, StdError, Uint128};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    BidPoolResponse, BidResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};

#[test]
fn one_bidder_distribution() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
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
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(3000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE BIDS 100 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 1u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    let info = mock_info("alice0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 2 COL AT  30UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(2u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE CAN CLAIM 2 COL
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "2"),
        ]
    );

    // ALICE CAN ONLY WITHDARW 40 UST (SPENT 59 UST 1% discount)
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "41"),
        ]
    );
}

#[test]
fn two_bidder_distribution() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(1000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE BIDS 100 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 4 COL AT  10UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(4u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // BOB BIDS 60 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(60u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // CHANGE COL PRICE TO 20 UST/COL
    let info = mock_info("col0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(2000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    // EXECUTE 6 COL AT 20 UST/COL
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(6u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE:
    //      SPENT: 40 UST + 60 UST
    //      CLAIM: 4col + 3col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "7"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );

    // BOB:
    //      SPENT: 60 UST (remaining 20)
    //      CLAIM: 3col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "3"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );
}

#[test]
fn two_bidder_distribution_big_numbers() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(1000000000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE BIDS 10,000 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 400 COL AT  10UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(400u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // BOB BIDS 6,000 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(6000000000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // CHANGE COL PRICE TO 20 UST/COL
    let info = mock_info("col0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(2000000000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    // EXECUTE 600 COL AT 20 UST/COL
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(600u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE:
    //      SPENT: 4000 UST + 6000 UST
    //      CLAIM: 400col + 300col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "700"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );

    // BOB:
    //      SPENT: 6000 UST (remaining 2000)
    //      CLAIM: 300col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "300"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );
}

#[test]
fn one_user_two_bid_slots() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 10u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(1000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE BIDS 100 UST at 5%
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // ALICE BIDS 100 UST at 10%
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100000000u128),
        }],
    );
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128), Uint128::from(2u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 5 COL AT  10UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(5000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE CAN CLAIM 5 COL
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "5000000"),
        ]
    );

    // EXECUTE 10 COL AT  10UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(10000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE CAN CLAIM FROM ALL BIDS
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "9999999"), // rounding, favors the system
        ]
    );

    // ALICE WITHDRAWS FROM 5% BID - FAIL
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();

    //  WITHDRAW FROM 10% BID
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "2"),
            attr("amount", "59736835"), // 100 ust - 40.263165 = 59.736835 UST
        ]
    );
}

#[test]
fn partial_withdraw_after_execution() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(5000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE BIDS 1000 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 10 COL AT  50UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(10u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE WITHDRAWS 250 UST
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(250u64)),
    };
    let info = mock_info("alice0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // BOB BIDS 250 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(250u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 4 COL AT 50 UST/COL
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(4u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let info = mock_info("col0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE:
    //      WITHDRAWABLE: 150UST
    //      CLAIM: 10col + 2col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "12"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "150"),
        ]
    );
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );

    // BOB:
    //      WITHDRAWABLE: 150UST
    //      CLAIM: 2col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "2"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "2"),
            attr("amount", "150"),
        ]
    );
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );
}

#[test]
fn completely_empty_pool() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(5000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE BIDS 1000 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // EXECUTE 20 COL AT  50UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(20u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // BOB BIDS 2000 UST
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 0u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let bid_response: BidResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Bid {
                bid_idx: Uint128::from(2u128),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(!bid_response.product_snapshot.is_zero(),);
    assert!(bid_response.epoch_snapshot == Uint128::from(1u128)); // epoch increased

    let bid_pool: BidPoolResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidPool {
                collateral_token: "col0000".to_string(),
                bid_slot: 0u8,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        bid_pool,
        BidPoolResponse {
            sum_snapshot: Decimal256::zero(),    // reseted
            product_snapshot: Decimal256::one(), // reseted
            premium_rate: Decimal256::zero(),
            total_bid_amount: Uint256::from(2000u128), // only bob's bid
            current_epoch: Uint128::from(1u128),       // increased epoch
            current_scale: Uint128::zero(),
        }
    );

    // EXECUTE 20 COL AT  50UST/COL
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(20u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // alice can only claim the initial 20 col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "20"),
        ]
    );
    // alice can't withdraw, bid is consumed
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    // bob can claim the later 20 col
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "20"),
        ]
    );
}

#[test]
fn product_truncated_to_zero() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(100),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // force product to become zero
    for _ in 0..8 {
        // ALICE BIDS 1000000000 uUST
        let msg = ExecuteMsg::SubmitBid {
            collateral_token: "col0000".to_string(),
            premium_slot: 0u8,
        };
        let info = mock_info(
            "alice0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000000u128),
            }],
        );
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // EXECUTE 999999995 COL AT  1 UST/COL
        let info = mock_info("col0000", &[]);
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "custody0000".to_string(),
            amount: Uint128::from(999999995u128), // 5 uusd residue
            msg: to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: "liquidator00000".to_string(),
                fee_address: Some("fee0000".to_string()),
                repay_address: Some("repay0000".to_string()),
            })
            .unwrap(),
        });
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    // alice can claim everything
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "7999999959"), // 999999995 * 8 = 7,999,999,960 missing 1ucol due to rounding and product resolution
        ]
    );

    let bid_pool: BidPoolResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BidPool {
                collateral_token: "col0000".to_string(),
                bid_slot: 0u8,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(bid_pool.total_bid_amount, Uint256::from(40u128)); // 5 * 8 = 40

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(8u128), // only last bid is active, others are consumed
        amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "8"),
            attr("amount", "39"), // 5 * 8 = 40 missing 1ucol due to rounding
        ]
    );
}

#[test]
// Test 1
// Two bidder reward distribution on a common slot
fn two_bidder_reward_distribution_common_slot() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    // 10 ust/col
    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(1000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE BIDS 100 UST IN THE 5% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // BOB BIDS 100 UST IN THE SAME POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 10 COL AT  9.5 UST/COL
    //  Executed col: 10
    //  Spent: 95 ust
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(10u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE:
    //      SPENT: 95 / 2 = 47.5 ust
    //      CLAIM: 5 col
    //      WITHDRAW: 100 - 47.5 = 52.5 ust
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "5"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(52u64)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "52"),
        ]
    );

    // BOB:
    //      SPENT: 95 / 2 = 47.5 UST
    //      CLAIM: 5 col
    //      WITHDRAW: 100 - 47.5 = 52.5 UST
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "5"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: Some(Uint256::from(52u128)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "2"),
            attr("amount", "53"),
        ]
    );
}

#[test]
// Test 2: two bidder reward distribution on multiple common slots
fn two_bidder_distribution_multiple_common_slots() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
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
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(200),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };

    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Alice BIDS 100 UST to 5% pool
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };

    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    let info = mock_info("alice0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // BOB BIDS 100 UST TO THE 5% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };

    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE BIDS 200 UST TO THE 10% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(3u128)]),
    };

    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // Bob Bids 200 UST to 10% pool
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(4u128)]),
    };

    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // 5% pool: Executes 10 collaterals at 9.5 ust/col
    //  Executed Collateral: 10 col
    //  Total spent: 95 ust
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(10u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // 10% pool: executes 22 collaterals at 9 ust/col
    //  Executed Collateral: 22
    //  Total Spent: 198 ust
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(22u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // bidders claiming the collaterals
    //  Alice: 5 col from the 5% pool, 11 col from the 10% pool
    //  Bob: 5 col from the 5% pool, 11 col from the 10% pool

    // ALICE LIQUIDATION CLAIM
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };

    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "15"),
        ]
    );

    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(3u128)]),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "0"),
        ]
    );

    // BOB LIQUIDATION CLAIM
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "16"),
        ]
    );
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(4u128)]),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "0"),
        ]
    );

    // RetractBid Withdrawal Claims
    //  Alice: 2.5 ust from the 5% pool, 1 ust from the 10% pool
    //  Bob: 2.5 ust from the 5% pool, 1 ust from the 10% pool

    // ALICE WITHDRAWALS from bid_idx 1, 3
    let info = mock_info("alice0000", &[]);

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(2u128)),
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "2"),
        ]
    );

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(3u128),
        amount: Some(Uint256::from(1u128)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "3"),
            attr("amount", "1"),
        ]
    );

    // BOB WITHDRAWALS from bid_idx 2, 4
    let info = mock_info("bob0000", &[]);

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: Some(Uint256::from(2u128)),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "2"),
            attr("amount", "2"),
        ]
    );

    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(4u128),
        amount: Some(Uint256::from(1u128)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "4"),
            attr("amount", "1"),
        ]
    );
}

#[test]
// Test 3
// two bidder unequal deposit reward distribution on a common slot
fn two_bidder_unequal_deposit_reward_distribution() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    // 2 ust/col
    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(200),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE BIDS 150 UST IN THE 2% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 2u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(150u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // BOB BIDS 200 UST IN THE SAME POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 2u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 51 COL AT  1.96 UST/COL
    //  Executed col: 51
    //  Spent: 99.96
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(51u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE:
    //      STAKE: 3/7
    //      Reward: 51 * 3/7 = 21.857...
    //      CLAIM: ???
    // BOB:
    //      STAKE: 4/7
    //      Reward: 51 * 4/7 = 29.1428...
    //      CLAIM: ???
    // WITHDRAWAL:
    //      Remaining bid pool: 350 - 99.96 = 250.04
    //      ALICE: 250.04 * 3/7 = 107.16
    //      BOB: 250.04 * 4/7 = 142.88

    // ALICE
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "21"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(107u64)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "107"),
        ]
    );

    // BOB
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "29"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: Some(Uint256::from(142u128)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "2"),
            attr("amount", "143"),
        ]
    );
}

// Test 4 Scalable Reward distribution after multiple liquidation events with changing stakes
#[test]
fn scalable_reward_distribution_after_multiple_liquidations() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
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
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(200),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };

    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE BIDS 50 UST TO 10% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(50u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // BOB BIDS 100 UST TO 10% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    let info = mock_info("bob0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // JOHN BIDS 100 UST TO 10% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "john0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(3u128)]),
    };

    let info = mock_info("john0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // FIRST LIQUIDATION EVENT
    // 10% POOL:
    //      Executed collaterals: 100

    let info = mock_info("col0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE DOES NOT MAKE ANY ADDITIONAL DEPOSITS AT THIS POINT

    // BOB AND JOHN EACH ADDS 250 UST TO THE 10% POOL

    // BOB BIDS 250 UST TO 10% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(250u128),
        }],
    );
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(4u128)]),
    };

    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // JOHN BIDS 250 UST TO 10% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 10u8,
    };

    let info = mock_info(
        "john0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(250u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(5u128)]),
    };

    let info = mock_info("john0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // SECOND LIQUIDATION EVENT
    // 10% POOL
    //      Executed collaterals: 50
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(50u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE CLAIMS COLLATERALS AND RETRACTS BID
    //  Alice's running sum of collateral reward: 21.1824
    //  Alice's remaining bid: 8.088

    // ALICE LIQUIDATION CLAIM
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };

    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "21"),
        ]
    );

    // ALICE WITHDRAWALS
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(8u128)),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "8"),
        ]
    );
}

// Test 5 Not enough bid pool to liquidate all collateral
//      Expected Behavior: Execute Liquidation returns an error if not all collateral was liquidated.
#[test]
fn not_enough_bid_for_collateral() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(300),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ),
    )]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), env, info, msg).unwrap();

    // ALICE BIDS 100 UST IN THE 6% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 6u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let info = mock_info("alice0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // BOB BIDS 100 UST IN THE 6% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 6u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let info = mock_info("bob0000", &[]);
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // TRY TO EXECUTE 100 COL AT  3 UST/COL
    // TOTAL COLLATERAL VALUE: 300 UST
    // TOTAL BID POOL AMOUNT: 200 UST
    // SHOULD RETURN AN ERROR
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("Not enough bids to execute this liquidation")
    )
}

#[test]
// Test 6
// Two bidder reward distribution on a common slot with large numbers
fn two_bidder_reward_distribution_common_slot_large_numbers() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    // 1000 ust/col
    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(100000000000),
            env.block.time.seconds(),
            env.block.time.seconds(),
        ), // 1000 ust/col
    )]);

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE BIDS 1 TRILLION UST IN THE 5% POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000000000000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // BOB BIDS 1 TRILLION UST IN THE SAME POOL
    let msg = ExecuteMsg::SubmitBid {
        collateral_token: "col0000".to_string(),
        premium_slot: 5u8,
    };
    let info = mock_info(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000000000000000u128),
        }],
    );
    let env = mock_env();
    let wait_end = env.block.time.plus_seconds(60u64);
    execute(deps.as_mut(), env, info.clone(), msg).unwrap();

    let msg = ExecuteMsg::ActivateBids {
        collateral_token: "col0000".to_string(),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let mut env = mock_env();
    env.block.time = wait_end;
    execute(deps.as_mut(), env, info, msg).unwrap();

    // EXECUTE 1 BILLION COL AT 950 UST/COL (1000 * 0.95)
    //  Executed col: 1 BILLION
    //  Spent: 950 BILLION ust
    let info = mock_info("col0000", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "custody0000".to_string(),
        amount: Uint128::from(1000000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteBid {
            liquidator: "liquidator00000".to_string(),
            fee_address: Some("fee0000".to_string()),
            repay_address: Some("repay0000".to_string()),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // ALICE:
    //      SPENT: 950 billion / 2 = 475 billion ust
    //      CLAIM: 500 million col
    //      WITHDRAW: 1.05 trillion / 2 = 0.525 billion = 525 million
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("alice0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "500000000"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(525000000000000u64)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "1"),
            attr("amount", "525000000000000"),
        ]
    );

    // BOB:
    //      SPENT: 950 billion / 2 = 475 billion ust
    //      CLAIM: 500 million col
    //      WITHDRAW: 1.05 trillion / 2 = 0.525 billion = 525 million
    let msg = ExecuteMsg::ClaimLiquidations {
        collateral_token: "col0000".to_string(),
        bids_idx: None,
    };
    let info = mock_info("bob0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_liquidations"),
            attr("collateral_token", "col0000"),
            attr("collateral_amount", "500000000"),
        ]
    );
    let msg = ExecuteMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: Some(Uint256::from(525000000u128)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "retract_bid"),
            attr("bid_idx", "2"),
            attr("amount", "525000000"),
        ]
    );
}
