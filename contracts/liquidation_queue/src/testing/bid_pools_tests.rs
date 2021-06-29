use crate::contract::{handle, init};
use crate::testing::mock_querier::{mock_dependencies, mock_env_with_block_time};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{log, to_binary, Coin, Decimal, HumanAddr, StdError, Uint128};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{Cw20HookMsg, HandleMsg, InitMsg};

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
            log("action", "calim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "2"),
        ]
    );

    // ALICE CAN ONLY WITHDARW 40 UST (SPENT 59 UST 1% discount)
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
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    // ALICE BIDS 100 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 10u8,
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
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env.clone(), msg).unwrap();

    // BOB BIDS 80 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 10u8,
    };
    let env = mock_env_with_block_time(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(80u128),
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
            log("action", "calim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "7"),
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
            log("amount", "10"),
        ]
    );
    let res = handle(&mut deps, env, msg).unwrap_err();
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
            log("action", "calim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "3"),
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
            log("amount", "26"),
        ]
    );
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
            amount: Uint128::from(100u128),
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
            amount: Uint128::from(100u128),
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
        amount: Uint128::from(5u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
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
            log("action", "calim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "5"),
        ]
    );

    // EXECUTE 10 COL AT  10UST/COL
    let env = mock_env_with_block_time("col0000", &[], 101u64);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(10u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
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
            log("action", "calim_liquidations"),
            log("collateral_token", "col0000"),
            log("collateral_amount", "10"),
        ]
    );

    // ALICE WITHDRAWS FROM 5% BID - FAIL
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
            log("amount", "0"),
        ]
    );

    // now its eliminated
    let _res = handle(&mut deps, env.clone(), msg).unwrap_err();

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
            log("amount", "55"),
        ]
    );
}

// #[test]
// fn skip_saiting_period() {
//     let mut deps = mock_dependencies(20, &[]);
//     deps.querier.with_tax(
//         Decimal::percent(1),
//         &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
//     );

//     let msg = InitMsg {
//         owner: HumanAddr::from("owner0000"),
//         oracle_contract: HumanAddr::from("oracle0000"),
//         stable_denom: "uusd".to_string(),
//         safe_ratio: Decimal256::percent(10),
//         bid_fee: Decimal256::percent(1),
//         liquidation_threshold: Uint256::from(100000000u64),
//         price_timeframe: 101u64,
//         waiting_period: 60u64,
//     };

//     let env = mock_env("addr0000", &[]);
//     deps.querier.with_oracle_price(&[(
//         &("col0000".to_string(), "uusd".to_string()),
//         &(Decimal256::percent(1000), env.block.time, env.block.time),
//     )]);

//     let _res = init(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::WhitelistCollateral {
//         collateral_token: HumanAddr::from("col0000"),
//         max_slot: 30u8,
//         bid_threshold: Uint256::from(10u128),
//     };
//     let env = mock_env("owner0000", &[]);
//     handle(&mut deps, env, msg).unwrap();

//     // ALICE BIDS 10 UST
//     let msg = HandleMsg::SubmitBid {
//         collateral_token: HumanAddr::from("col0000"),
//         premium_slot: 1u8,
//     };
//     let env = mock_env(
//         "alice0000",
//         &[Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(10u128),
//         }],
//     );
//     let wait_end = env.block.time + 60u64;
//     handle(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::ActivateBids {
//         collateral_token: HumanAddr::from("col0000"),
//         bids_idx: Some(vec![Uint128::from(1u128)]),
//     };
//     let env = mock_env_with_block_time("alice0000", &[], wait_end - 5u64); // before wait_end
//     let err = handle(&mut deps, env, msg.clone()).unwrap_err(); // expect error
//     assert_eq!(
//         err,
//         StdError::generic_err(format!("Wait period expires at {}", wait_end))
//     );

//     // succeed
//     let env = mock_env_with_block_time("alice0000", &[], wait_end);
//     handle(&mut deps, env, msg).unwrap();

//     // set custody collateral balance to 100
//     deps.querier.with_token_balances(&[(
//         &HumanAddr::from("col0000"),
//         &[(&HumanAddr::from("custody0000"), &Uint128::from(100u128))],
//     )]);

//     // REPEAT FOR BID OF 90 UST // since custody balance is 100, current bid is equal to 1%, so wait period is enforced
//     let msg = HandleMsg::SubmitBid {
//         collateral_token: HumanAddr::from("col0000"),
//         premium_slot: 1u8,
//     };
//     let env = mock_env(
//         "alice0000",
//         &[Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(90u128),
//         }],
//     );
//     let wait_end = env.block.time + 60u64;
//     handle(&mut deps, env, msg.clone()).unwrap();

//     let msg = HandleMsg::ActivateBids {
//         collateral_token: HumanAddr::from("col0000"),
//         bids_idx: Some(vec![Uint128::from(2u128)]),
//     };
//     let env = mock_env_with_block_time("alice0000", &[], wait_end);
//     handle(&mut deps, env, msg).unwrap();

//     // set custody collateral balance to 1000
//     deps.querier.with_token_balances(&[(
//         &HumanAddr::from("col0000"),
//         &[(&HumanAddr::from("custody0000"), &Uint128::from(20000u128))],
//     )]);

//     // BID IS DIRECTLY ACTIVATED
//     let msg = HandleMsg::SubmitBid {
//         collateral_token: HumanAddr::from("col0000"),
//         premium_slot: 1u8,
//     };
//     let env = mock_env(
//         "alice0000",
//         &[Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(90u128),
//         }],
//     );
//     let wait_end = env.block.time + 60u64;
//     handle(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::ActivateBids {
//         collateral_token: HumanAddr::from("col0000"),
//         bids_idx: Some(vec![Uint128::from(3u128)]),
//     };
//     let env = mock_env_with_block_time("alice0000", &[], wait_end);
//     let err = handle(&mut deps, env, msg.clone()).unwrap_err(); // expect error
//     assert_eq!(err, StdError::generic_err("Bid is already active"));
// }

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
            log("action", "calim_liquidations"),
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
            log("action", "calim_liquidations"),
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
fn withdraw_removed_share_max() {
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

    // EXECUTE 15 COL AT  50UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(15u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
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

    // EXECUTE 40 COL AT  50UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(40u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env, msg).unwrap();

    // CINDY BIDS 3000 UST
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 0u8,
    };
    let env = mock_env(
        "bob0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(3000u128),
        }],
    );
    handle(&mut deps, env, msg.clone()).unwrap();

    // EXECUTE 50 COL AT  50UST/COL
    let env = mock_env("col0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(50u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    handle(&mut deps, env, msg).unwrap();

    let env = mock_env("alice0000", &[]);
    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(7u64)),
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "1"),
            log("amount", "7"),
        ]
    );
}
