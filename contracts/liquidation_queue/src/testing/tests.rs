use crate::contract::{handle, init, query};
use crate::testing::mock_querier::{mock_dependencies, mock_env_with_block_time};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, log, to_binary, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128,
};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    BidResponse, ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg,
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
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
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
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
            waiting_period: 60u64,
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
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // update owner
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: Some(HumanAddr("owner0001".to_string())),
        oracle_contract: None,
        stable_denom: None,
        safe_ratio: None,
        bid_fee: None,
        liquidation_threshold: None,
        price_timeframe: None,
        waiting_period: None,
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
            liquidation_threshold: Uint256::from(100000000u64),
            price_timeframe: 60u64,
            waiting_period: 60u64,
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
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(120u64),
        waiting_period: Some(100u64),
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
            liquidation_threshold: Uint256::from(150000000u64),
            price_timeframe: 120u64,
            waiting_period: 100u64,
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
        liquidation_threshold: Some(Uint256::from(150000000u64)),
        price_timeframe: Some(100u64),
        waiting_period: Some(100u64),
    };

    let err = handle(&mut deps, env, msg).unwrap_err();
    assert_eq!(err, StdError::unauthorized());
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
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("asset0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_slot: 1u8,
    };
    let env = mock_env("addr0000", &[]);
    let err = handle(&mut deps, env, msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("No uusd assets have been provided")
    );

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg.clone()).unwrap();

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
            collateral_token: HumanAddr::from("asset0000"),
            owner: HumanAddr::from("addr0000"),
            amount: Uint256::from(1000000u128),
            premium_slot: 1u8,
            spent: Uint256::zero(),
            pending_liquidated_collateral: Uint256::zero(),
            share: Uint256::zero(),
            wait_end: Some(wait_end),
        }
    );
}

#[test]
fn activate_bid() {
    let mut deps = mock_dependencies(20, &[]);

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
        collateral_token: HumanAddr::from("asset0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_slot: 1u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("asset0000"),
        bids_idx: Some(vec![Uint128::from(1u64)]),
    };
    let env = mock_env_with_block_time("addr0001", &[], wait_end);
    let err = handle(&mut deps, env, msg.clone()).unwrap_err();
    assert_eq!(err, StdError::unauthorized());

    let env = mock_env_with_block_time("addr0000", &[], wait_end - 2u64);
    let err = handle(&mut deps, env, msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(format!("Wait period expires at {}", wait_end))
    );

    let env = mock_env_with_block_time("addr0000", &[], wait_end);
    handle(&mut deps, env, msg.clone()).unwrap(); // TODO: check messages

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
            collateral_token: HumanAddr::from("asset0000"),
            owner: HumanAddr::from("addr0000"),
            amount: Uint256::from(1000000u128),
            premium_slot: 1u8,
            spent: Uint256::zero(),
            pending_liquidated_collateral: Uint256::zero(),
            share: Uint256::from(1u128),
            wait_end: None,
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
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 60u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("asset0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_slot: 1u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg.clone()).unwrap();

    // let msg = HandleMsg::RetractBid {
    //     bid_idx: Uint128::from(1u128),
    //     amount: None,
    // };
    // let env = mock_env("addr0000", &[]);
    // let err = handle(&mut deps, env, msg).unwrap_err();
    // assert_eq!(err, StdError::generic_err("Bid is not active"));

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("asset0000"),
        bids_idx: Some(vec![Uint128::from(1u64)]),
    };
    let env = mock_env_with_block_time("addr0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: None,
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: HumanAddr::from("addr0000"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1000000u128),
            }]
        })]
    );
}

#[test]
fn retract_unactive_bid() {
    let mut deps = mock_dependencies(20, &[]);

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
        collateral_token: HumanAddr::from("asset0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_slot: 1u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::RetractBid {
        bid_idx: Uint128::from(1u128),
        amount: Some(Uint256::from(500000u128)),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "retract_bid"),
            log("bid_idx", "1"),
            log("amount", "500000"),
        ]
    );

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("asset0000"),
        bids_idx: None,
    };
    let env = mock_env_with_block_time("addr0000", &[], wait_end);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![log("action", "activate_bids"), log("amount", "500000"),]
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
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 100000u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(50), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("asset0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_slot: 1u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("asset0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("addr0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // required_stable 495,000
    // bid_fee         4,950
    // repay_amount    490,050
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(1000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    let env = mock_env("asset0000", &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("repay0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(485198u128), // 490050 / (1 + tax_rate)
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("fee0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            }),
        ]
    );

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(1000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: None,
                repay_address: None,
            })
            .unwrap(),
        ),
    });
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("addr0001"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(485198u128), // 490050 / (1 + tax_rate)
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("addr0001"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(4900u128), // 4950 / (1 + tax_rate)
                }]
            }),
        ]
    );

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(2020206u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    let res = handle(&mut deps, env, msg).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("Not enough bids to execute this liquidation")
    );
}

#[test]
fn claim_liquidations() {
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
        price_timeframe: 1000000u64,
        waiting_period: 60u64,
    };

    let env = mock_env("addr0000", &[]);
    deps.querier.with_oracle_price(&[(
        &("asset0000".to_string(), "uusd".to_string()),
        &(Decimal256::percent(50), env.block.time, env.block.time),
    )]);

    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::WhitelistCollateral {
        collateral_token: HumanAddr::from("asset0000"),
        max_slot: 30u8,
        bid_threshold: Uint256::zero(),
    };
    let env = mock_env("owner0000", &[]);
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SubmitBid {
        collateral_token: HumanAddr::from("asset0000"),
        premium_slot: 1u8,
    };
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let wait_end = env.block.time + 60u64;
    handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::ActivateBids {
        collateral_token: HumanAddr::from("asset0000"),
        bids_idx: Some(vec![Uint128::from(1u128)]),
    };
    let env = mock_env_with_block_time("addr0000", &[], wait_end);
    handle(&mut deps, env, msg).unwrap();

    // required_stable 495,000
    // bid_fee         4,950
    // repay_amount    490,050
    let env = mock_env("asset0000", &[]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0001"),
        amount: Uint128::from(1000000u128),
        msg: Some(
            to_binary(&Cw20HookMsg::ExecuteBid {
                liquidator: HumanAddr::from("liquidator00000"),
                fee_address: Some(HumanAddr::from("fee0000")),
                repay_address: Some(HumanAddr::from("repay0000")),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::ClaimLiquidations {
        collateral_token: HumanAddr::from("asset0000"),
        bids_idx: None,
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "calim_liquidations"),
            log("collateral_token", "asset0000"),
            log("collateral_amount", "1000000"),
        ]
    );
}
