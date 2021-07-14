use crate::contract::{handle, init, query};
use crate::testing::mock_querier::{mock_dependencies, mock_env_with_block_time};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{from_binary, log, to_binary, Coin, Decimal, HumanAddr, StdError, Uint128};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    BidPoolResponse, BidResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg,
};

// The overall contract-level flow of liquidation queue 
// LQ queries Overseer -> receives collateral info -> LQ simulates auction -> LQ executes auction 

#[test]
fn two_bidder_distribution_common_slots() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        oracle_contract: HumanAddr::from("oracle0000"),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10), // why 0.1? 
        bid_fee: Decimal256::percent(1),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64, // units?
        waiting_period: 60u64 // units? 
    };

    let env = mock_env("addr0000", &[]); // first env: just random address? 
    deps.querier.with_oracle_price(&[( // returning custom collateral oracle price
        &("col0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(3000), env.block.time, env.block.time),
    )]);  // 30 uusd/col? 

    let _res = init(&mut deps, env, msg).unwrap(); // when is _res used again? initializing what?
    println!(&_res);

    let msg = HandleMsg::WhitelistCollateral { // does this mean its listing it on the auction?
        collateral_token: HumanAddr::from("col0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
        premium_rate_per_slot: Decimal256::percent(1), // 1 2 3 4? always in equal intervals?
    };
    
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap(); // handle: execute msg to the LQ contract? 

    // Alice BIDS 100 UST to slot:1 
    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("col0000"),
        premium_slot: 1u8,
    };

    let env = mock_env_with_block_time( // why do we keep updating the env variable? defining on chain state?
        "alice0000",
        &[Coin {
            denom: "uusd".to_sring(),
            amount: Uint128::from(100u128), // does this need to be updated with every action?
        }],
        064,
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap(); // execute msg to the LQ contract? 

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("col0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };

    let env = mock_env_with_block_time("alice0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap(); 

    // Execute collaterals 
    let env = mock_env_with_block_time("col0000", &[], 101u64); // mocking a state of a chain? 
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"), // who is the sender? 
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

    // bidders claiming the collaterals
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
    )

    // Bidders withdrawing amount leftover 
    // how come env is not redefined here? 
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