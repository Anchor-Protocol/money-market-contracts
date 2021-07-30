use crate::contract::{handle, init, query};
use crate::testing::mock_querier::{mock_dependencies, mock_env_with_block_time};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{from_binary, log, to_binary, Coin, Decimal, HumanAddr, StdError, Uint128};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    BidPoolResponse, BidResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg,
};

#[test]
fn one_bidder_distribution() {
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
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(3000), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 100 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 1u8,
    };
    let env = mock_env_with_block_time(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
        0u64,
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("alice0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // EXECUTE 2 COL AT  30UST/COL
    let env = mock_env_with_block_time("col0000", &[], 101u64);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(2u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE CAN CLAIM 2 COL
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("alice0000", &[], 101u64);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "2"),
        ]
    );

    // ALICE CAN ONLY WITHDRAW 40 UST (SPENT 59 UST 1% discount)
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "1"),
            log("amount", "41"),
        ]
    );
}

#[test]
fn two_bidder_distribution() {
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
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(1000), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 100 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("alice0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // EXECUTE 4 COL AT  10UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(4u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // BOB BIDS 60 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env_with_block_time(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(60u128),
        }],
        101u64,
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let env = mock_env_with_block_time("bob0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // CHANGE COL PRICE TO 20 UST/COL
    let env = mock_env("col0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(2000), env.block.time, env.block.time),
    )]);

    // EXECUTE 6 COL AT 20 UST/COL
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(6u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE:
    //      SPENT: 40 UST + 60 UST
    //      CLAIM: 4col + 3col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env("alice0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "7"),
        ]
    );
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );

    // BOB:
    //      SPENT: 60 UST (remaining 20)
    //      CLAIM: 3col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env("bob0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "3"),
        ]
    );
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );
}

#[test]
fn two_bidder_distribution_big_numbers() {
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
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(1000000000),
            env.block.time,
            env.block.time,
        ),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 10,000 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("alice0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // EXECUTE 400 COL AT  10UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(400u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // BOB BIDS 6,000 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env_with_block_time(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(6000000000u128),
        }],
        101u64,
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let env = mock_env_with_block_time("bob0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // CHANGE COL PRICE TO 20 UST/COL
    let env = mock_env("col0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            Decimal256::percent(2000000000),
            env.block.time,
            env.block.time,
        ),
    )]);

    // EXECUTE 600 COL AT 20 UST/COL
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(600u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE:
    //      SPENT: 4000 UST + 6000 UST
    //      CLAIM: 400col + 300col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env("alice0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "700"),
        ]
    );
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = handle(&mut deps, env, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );

    // BOB:
    //      SPENT: 6000 UST (remaining 2000)
    //      CLAIM: 300col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env("bob0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "300"),
        ]
    );
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = handle(&mut deps, env, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );
}

#[test]
fn one_user_two_bid_slots() {
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
        price_timeframe: 10u64,
        waiting_period: 60u64,
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(1000), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 100 UST at 5%
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 5u8,
    };
    let env = mock_env(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE BIDS 100 UST at 10%
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 10u8,
    };
    let env = mock_env(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(100000000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128), Uint128::from(2u128)]),
    };
    let env = mock_env_with_block_time("alice0000", &[], wait_end);
    handle(&mut deps, env.clone(), msg).unwrap();

    // EXECUTE 5 COL AT  10UST/COL
    let env = mock_env_with_block_time("col0000", &[], 101u64);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(5000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE CAN CLAIM 5 COL
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env_with_block_time("alice0000", &[], 101u64);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "5000000"),
        ]
    );

    // EXECUTE 10 COL AT  10UST/COL
    let env = mock_env_with_block_time("col0000", &[], 101u64);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(10000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE CAN CLAIM FROM ALL BIDS
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env_with_block_time("alice0000", &[], 101u64);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "9999999"), // rounding, favors the system
        ]
    );

    // ALICE WITHDRAWS FROM 5% BID - FAIL
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    handle(&mut deps, env.clone(), msg.clone()).unwrap_err();

    //  WITHDRAW FROM 10% BID
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "2"),
            log("amount", "59736835"), // 100 ust - 40.263165 = 59.736835 UST
        ]
    );
}

#[test]
fn partial_withdraw_after_execution() {
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
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(5000), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 1000 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("alice0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // EXECUTE 10 COL AT  50UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(10u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // ALICE WITHDRAWS 250 UST
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(250u64)),
    };
    let env = mock_env("alice0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // BOB BIDS 250 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(250u128),
        }],
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(2u128)]),
    };
    let env = mock_env_with_block_time("bob0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // EXECUTE 4 COL AT 50 UST/COL
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(4u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    let env = mock_env("col0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE:
    //      WITHDRAWABLE: 150UST
    //      CLAIM: 10col + 2col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env("alice0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "12"),
        ]
    );
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "1"),
            log("amount", "150"),
        ]
    );
    let res = handle(&mut deps, env, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );

    // BOB:
    //      WITHDRAWABLE: 150UST
    //      CLAIM: 2col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env("bob0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "2"),
        ]
    );
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(2u128),
        amount: None,
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "2"),
            log("amount", "150"),
        ]
    );
    let res = handle(&mut deps, env, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("No bids with the specified information exist")
    );
}

#[test]
fn completely_empty_pool() {
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
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(5000), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 1000 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "alice0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    // EXECUTE 20 COL AT  50UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(20u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env, msg).unwrap();

    // BOB BIDS 2000 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000u128),
        }],
    );
    handle(&mut deps, env, msg).unwrap();

    let bid_response: BidResponse = from_binary(
        &query(
            &deps,
            QueryMsg::Bid {
                bid_idx: Uint128::from(2u128),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert!(!bid_response.product_snapshot.is_zero(),);
    assert!(bid_response.epoch_snapshot == Uint128(1)); // epoch increased

    let bid_pool: BidPoolResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidPool {
                collateral_token: HumanAddr::from("col0000"),
                bid_slot: 0u8,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        bid_pool,
        BidPoolResponse {
            sum_snapshot: Decimal256::zero(),    // reset
            product_snapshot: Decimal256::one(), // reset
            premium_rate: Decimal256::zero(),
            total_bid_amount: Uint256::from(2000u128), // only bob's bid
            current_epoch: Uint128(1),                 // increased epoch
            current_scale: Uint128::zero(),
        }
    );

    // EXECUTE 20 COL AT  50UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(20u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env, msg).unwrap();

    // alice can only claim the initial 20 col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env("alice0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "20"),
        ]
    );
    // alice can't withdrawn, bid is consumed
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    handle(&mut deps, env.clone(), msg.clone()).unwrap_err();

    // bob can claim the later 20 col
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env("bob0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "20"),
        ]
    );
}

#[test]
fn product_truncated_to_zero() {
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
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: HumanAddr::from("overseer0000"),
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(100), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // force product to become zero
    let mut total_liquidated = Uint256::zero();
    let mut remaining_bid = Uint256::zero();
    for _ in 0..8 {
        // ALICE BIDS 1000000000 uUST
        let msg = HandleMsg::SubmitBid {
            collateral_token: HumanAddr::from("col0000"),
            premium_slot: 0u8,
        };
        let env = mock_env(
            "alice0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000000u128),
            }],
        );
        handle(&mut deps, env, msg).unwrap();

        // EXECUTE 999999999 COL AT  1 UST/COL
        let env = mock_env("col0000", &[]);
        let msg = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("addr0001"),
            amount: Uint128::from(999999999u128),
            msg: Some(
                to_binary(&Cw20HookMsg::ExecuteBid {
                    liquidator: HumanAddr::from("liquidator00000"),
                    fee_address: Some(HumanAddr::from("fee0000")),
                    repay_address: Some(HumanAddr::from("repay0000")),
                })
                .unwrap(),
            ),
        });
        handle(&mut deps, env, msg).unwrap();
        total_liquidated += Uint256::from(999999999u128);
        remaining_bid += Uint256::one(); // 1000000000 - 999999999
    }

    // alice can claim everything
    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: None,
    };
    let env = mock_env("alice0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "claim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "7999999990"), // real expected 7999999992, missing 2uCol because as product gets smaller might loose some precision, but favor the system anyways
        ]
    );

    let bid_pool: BidPoolResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BidPool {
                collateral_token: HumanAddr::from("col0000"),
                bid_slot: 0u8,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(bid_pool.total_bid_amount, remaining_bid);

    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(8u128), // only last bid is active, others are consumed
        amount: None,
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "8"),
            log("amount", "7"), // system favors later bids, but never bigger than actual bid amount
        ]
    );
}
