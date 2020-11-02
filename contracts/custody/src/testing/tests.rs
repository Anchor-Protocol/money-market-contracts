use cosmwasm_std::{
    from_binary, log, to_binary, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128,
    WasmMsg,
};

use crate::contract::{handle, init, query};
use crate::external::handle::RewardContractHandleMsg;
use crate::msg::{BorrowerResponse, ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg};
use crate::state::increase_global_index;
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cw20::Cw20ReceiveMsg;
use terra_cosmwasm::create_swap_msg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    let query_res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(HumanAddr::from("bluna"), config_res.collateral_token);
    assert_eq!(HumanAddr::from("overseer"), config_res.overseer_contract);
    assert_eq!(HumanAddr::from("market"), config_res.market_contract);
    assert_eq!(HumanAddr::from("reward"), config_res.reward_contract);
    assert_eq!("uusd".to_string(), config_res.reward_denom);
}

#[test]
fn deposit_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0000"),
        amount: Uint128::from(100u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap()),
    });

    // failed; cannot directly execute receive message
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env("bluna", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_collateral"),
            log("borrower", "addr0000"),
            log("amount", "100"),
        ]
    );

    let query_res = query(
        &deps,
        QueryMsg::Borrower {
            address: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();

    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: HumanAddr::from("addr0000"),
            balance: Uint128::from(100u128),
            spendable: Uint128::from(100u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        }
    );

    // Check before_balance change
    increase_global_index(&mut deps.storage, Decimal::from_ratio(1000000u128, 1u128)).unwrap();
    let _res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_collateral"),
            log("borrower", "addr0000"),
            log("amount", "100"),
        ]
    );

    let query_res = query(
        &deps,
        QueryMsg::Borrower {
            address: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: HumanAddr::from("addr0000"),
            balance: Uint128::from(200u128),
            spendable: Uint128::from(200u128),
            reward_index: Decimal::from_ratio(1000000u128, 1u128),
            pending_reward: Uint128::from(100000000u128),
        }
    );
}

#[test]
fn withdraw_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    // Check reward index update
    increase_global_index(&mut deps.storage, Decimal::from_ratio(1000000u128, 1u128)).unwrap();

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0000"),
        amount: Uint128::from(100u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap()),
    });

    let env = mock_env("bluna", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_collateral"),
            log("borrower", "addr0000"),
            log("amount", "100"),
        ]
    );

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint128(110u128)),
    };

    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Cannot withdraw more than spendable balance 100")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint128(50u128)),
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_collateral"),
            log("borrower", "addr0000"),
            log("amount", "50"),
        ]
    );

    let query_res = query(
        &deps,
        QueryMsg::Borrower {
            address: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: HumanAddr::from("addr0000"),
            balance: Uint128::from(50u128),
            spendable: Uint128::from(50u128),
            reward_index: Decimal::from_ratio(1000000u128, 1u128),
            pending_reward: Uint128::zero(),
        }
    );

    let _res = handle(&mut deps, env, msg).unwrap();
    let query_res = query(
        &deps,
        QueryMsg::Borrower {
            address: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: HumanAddr::from("addr0000"),
            balance: Uint128::zero(),
            spendable: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        }
    );
}

#[test]
fn lock_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    // Check reward index update
    increase_global_index(&mut deps.storage, Decimal::from_ratio(1000000u128, 1u128)).unwrap();

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0000"),
        amount: Uint128::from(100u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap()),
    });

    let env = mock_env("bluna", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_collateral"),
            log("borrower", "addr0000"),
            log("amount", "100"),
        ]
    );

    let msg = HandleMsg::LockCollateral {
        borrower: HumanAddr::from("addr0000"),
        amount: Uint128::from(50u128),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env("overseer", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "lock_collateral"),
            log("borrower", "addr0000"),
            log("amount", "50"),
        ]
    );

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint128(51u128)),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Cannot withdraw more than spendable balance 50")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint128(50u128)),
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_collateral"),
            log("borrower", "addr0000"),
            log("amount", "50"),
        ]
    );

    let query_res = query(
        &deps,
        QueryMsg::Borrower {
            address: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: HumanAddr::from("addr0000"),
            balance: Uint128::from(50u128),
            spendable: Uint128::from(0u128),
            reward_index: Decimal::from_ratio(1000000u128, 1u128),
            pending_reward: Uint128::zero(),
        }
    );

    // Unlock partial amount of collateral
    let msg = HandleMsg::UnlockCollateral {
        borrower: HumanAddr::from("addr0000"),
        amount: Uint128::from(30u128),
    };
    let env = mock_env("overseer", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "unlock_collateral"),
            log("borrower", "addr0000"),
            log("amount", "30"),
        ]
    );

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint128(30u128)),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_collateral"),
            log("borrower", "addr0000"),
            log("amount", "30"),
        ]
    );

    let query_res = query(
        &deps,
        QueryMsg::Borrower {
            address: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: HumanAddr::from("addr0000"),
            balance: Uint128::from(20u128),
            spendable: Uint128::from(0u128),
            reward_index: Decimal::from_ratio(1000000u128, 1u128),
            pending_reward: Uint128::zero(),
        }
    );
}

#[test]
fn distribute_rewards() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(1000000u128),
        }],
    );

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::DistributeRewards {};
    let res = handle(&mut deps, env, msg).unwrap();
    // Do not print logs at this step
    assert_eq!(res.log, vec![]);
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("reward"),
                send: vec![],
                msg: to_binary(&RewardContractHandleMsg::WithdrawReward {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                send: vec![],
                msg: to_binary(&HandleMsg::SwapToRewardDenom {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                send: vec![],
                msg: to_binary(&HandleMsg::DistributeHook {
                    prev_balance: Uint128(1000000u128)
                })
                .unwrap(),
            }),
        ]
    );
}

#[test]
fn distribute_hook() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(1000000u128),
        }],
    );

    deps.querier.with_distribution_params(&[(
        &HumanAddr::from("bluna"),
        &(Decimal::percent(20), Decimal::percent(30)),
    )]);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("bluna"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(1000u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // Claimed rewards is 1000000uusd
    let msg = HandleMsg::DistributeHook {
        prev_balance: Uint128::zero(),
    };
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env(MOCK_CONTRACT_ADDR, &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "distribute_rewards"),
            log("borrower_rewards", "560000"),
            log("buffer_rewards", "240000"),
            log("depositer_subsidy", "200000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("market"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(198019u128),
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("overseer"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(237623u128)
                }],
            }),
        ],
    )
}

#[test]
fn swap_to_reward_denom() {
    let mut deps = mock_dependencies(
        20,
        &[
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128(1000000u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128(20000000000u128),
            },
            Coin {
                denom: "usdr".to_string(),
                amount: Uint128(2000000u128),
            },
        ],
    );

    let msg = InitMsg {
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        reward_denom: "uusd".to_string(),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SwapToRewardDenom {};
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env(MOCK_CONTRACT_ADDR, &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            create_swap_msg(
                HumanAddr::from(MOCK_CONTRACT_ADDR),
                Coin {
                    denom: "ukrw".to_string(),
                    amount: Uint128::from(20000000000u128),
                },
                "uusd".to_string()
            ),
            create_swap_msg(
                HumanAddr::from(MOCK_CONTRACT_ADDR),
                Coin {
                    denom: "usdr".to_string(),
                    amount: Uint128::from(2000000u128),
                },
                "uusd".to_string()
            ),
        ]
    );
}
