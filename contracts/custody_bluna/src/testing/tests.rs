use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    from_binary, log, to_binary, Api, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError,
    Uint128, WasmMsg,
};

use crate::contract::{handle, init, query};
use crate::external::handle::RewardContractHandleMsg;
use crate::state::read_borrower_info;
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use moneymarket::custody::{
    BAssetInfo, BorrowerResponse, ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg,
};
use moneymarket::liquidation::Cw20HookMsg as LiquidationCw20HookMsg;
use terra_cosmwasm::create_swap_msg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    let query_res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(HumanAddr::from("owner"), config_res.owner);
    assert_eq!(HumanAddr::from("bluna"), config_res.collateral_token);
    assert_eq!(HumanAddr::from("overseer"), config_res.overseer_contract);
    assert_eq!(HumanAddr::from("market"), config_res.market_contract);
    assert_eq!(HumanAddr::from("reward"), config_res.reward_contract);
    assert_eq!(
        HumanAddr::from("liquidation"),
        config_res.liquidation_contract
    );
    assert_eq!("uusd".to_string(), config_res.stable_denom);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::UpdateConfig {
        owner: Some(HumanAddr::from("owner2")),
        liquidation_contract: Some(HumanAddr::from("liquidation2")),
    };
    let env = mock_env("owner", &[]);
    handle(&mut deps, env, msg.clone()).unwrap();

    let query_res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(HumanAddr::from("owner2"), config_res.owner);
    assert_eq!(HumanAddr::from("bluna"), config_res.collateral_token);
    assert_eq!(HumanAddr::from("overseer"), config_res.overseer_contract);
    assert_eq!(HumanAddr::from("market"), config_res.market_contract);
    assert_eq!(HumanAddr::from("reward"), config_res.reward_contract);
    assert_eq!(
        HumanAddr::from("liquidation2"),
        config_res.liquidation_contract
    );
    assert_eq!("uusd".to_string(), config_res.stable_denom);

    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn deposit_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
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
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    //no messages sent
    let msg2 = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0000"),
        amount: Uint128::from(100u128),
        msg: None,
    });
    let res2 = handle(&mut deps, env.clone(), msg2.clone());
    match res2 {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Invalid request: \"deposit collateral\" message not included in request"
        ),
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
            balance: Uint256::from(100u128),
            spendable: Uint256::from(100u128),
        }
    );

    // Deposit more
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
            balance: Uint256::from(200u128),
            spendable: Uint256::from(200u128),
        }
    );
}

#[test]
fn withdraw_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

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
        amount: Some(Uint256::from(110u64)),
    };

    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(
                msg,
                "Withdraw amount cannot exceed the user's spendable amount: 100"
            )
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint256::from(50u64)),
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
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("bluna"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from("addr0000"),
                amount: Uint128::from(50u128),
            })
            .unwrap(),
        })]
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
            balance: Uint256::from(50u64),
            spendable: Uint256::from(50u64),
        }
    );

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint256::from(40u128)),
    };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
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
            balance: Uint256::from(10u128),
            spendable: Uint256::from(10u128),
        }
    );

    //withdraw with "None" amount
    let msg = HandleMsg::WithdrawCollateral { amount: None };
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
            balance: Uint256::zero(),
            spendable: Uint256::zero(),
        }
    );
}

#[test]
fn lock_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

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
        amount: Uint256::from(50u64),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    //locking more than spendable
    let env2 = mock_env("overseer", &[]);
    let msg2 = HandleMsg::LockCollateral {
        borrower: HumanAddr::from("addr0000"),
        amount: Uint256::from(200u128),
    };
    let res2 = handle(&mut deps, env2, msg2.clone()).unwrap_err();

    assert_eq!(
        res2,
        StdError::generic_err(format!(
            "Lock amount cannot excceed the user's spendable amount: 100"
        ))
    );

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

    //directly checking if spendable is decreased by amount
    let spend = read_borrower_info(
        &deps.storage,
        &deps
            .api
            .canonical_address(&HumanAddr::from("addr0000"))
            .unwrap(),
    )
    .spendable;
    assert_eq!(spend, Uint256::from(50u128));

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint256::from(51u64)),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(
                msg,
                "Withdraw amount cannot exceed the user's spendable amount: 50"
            )
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint256::from(50u64)),
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
            balance: Uint256::from(50u64),
            spendable: Uint256::from(0u64),
        }
    );

    // Unlock partial amount of collateral
    let msg = HandleMsg::UnlockCollateral {
        borrower: HumanAddr::from("addr0000"),
        amount: Uint256::from(30u64),
    };

    //unauthorized sender
    let env2 = mock_env("addr0000", &[]);
    let res2 = handle(&mut deps, env2, msg.clone());
    match res2 {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    //unlocking more than allowed (which is 50 - 0 = 50)
    let msg3 = HandleMsg::UnlockCollateral {
        borrower: HumanAddr::from("addr0000"),
        amount: Uint256::from(230u128),
    };

    let env3 = mock_env("overseer", &[]);
    let res3 = handle(&mut deps, env3, msg3);
    match res3 {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Unlock amount cannot exceed locked amount: 50")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

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

    //checking if amount is added to spendable
    let spend = read_borrower_info(
        &deps.storage,
        &deps
            .api
            .canonical_address(&HumanAddr::from("addr0000"))
            .unwrap(),
    )
    .spendable;
    assert_eq!(spend, Uint256::from(30u128));

    let msg = HandleMsg::WithdrawCollateral {
        amount: Some(Uint256::from(30u64)),
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
            balance: Uint256::from(20u64),
            spendable: Uint256::from(0u64),
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
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::DistributeRewards {};
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::DistributeRewards {};
    let env = mock_env("overseer", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    // Do not print logs at this step
    assert_eq!(res.log, vec![]);
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("reward"),
                send: vec![],
                msg: to_binary(&RewardContractHandleMsg::ClaimRewards { recipient: None }).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                send: vec![],
                msg: to_binary(&HandleMsg::SwapToStableDenom {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                send: vec![],
                msg: to_binary(&HandleMsg::DistributeHook {}).unwrap(),
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
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // Claimed rewards is 1000000uusd
    let msg = HandleMsg::DistributeHook {};
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
            log("buffer_rewards", "1000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: HumanAddr::from("overseer"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(990100u128)
            }],
        }),],
    )
}

#[test]
fn distribution_hook_zero_rewards() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("terraswap"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    // Claimed rewards is 1000000uusd
    let msg = HandleMsg::DistributeHook {};
    let env = mock_env(MOCK_CONTRACT_ADDR, &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "distribute_rewards"),
            log("buffer_rewards", "0"),
        ]
    );

    assert_eq!(res.messages, vec![],)
}

#[test]
fn swap_to_stable_denom() {
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
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::SwapToStableDenom {};
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
                "uusd".to_string(),
            ),
            create_swap_msg(
                HumanAddr::from(MOCK_CONTRACT_ADDR),
                Coin {
                    denom: "usdr".to_string(),
                    amount: Uint128::from(2000000u128),
                },
                "uusd".to_string(),
            ),
        ]
    );
}

#[test]
fn liquidate_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner"),
        collateral_token: HumanAddr::from("bluna"),
        overseer_contract: HumanAddr::from("overseer"),
        market_contract: HumanAddr::from("market"),
        reward_contract: HumanAddr::from("reward"),
        liquidation_contract: HumanAddr::from("liquidation"),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

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
        amount: Uint256::from(50u64),
    };
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

    let msg = HandleMsg::LiquidateCollateral {
        liquidator: HumanAddr::from("addr0001"),
        borrower: HumanAddr::from("addr0000"),
        amount: Uint256::from(100u64),
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env("overseer", &[]);
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Liquidation amount cannot exceed locked amount: 50")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
    let msg = HandleMsg::LiquidateCollateral {
        liquidator: HumanAddr::from("liquidator"),
        borrower: HumanAddr::from("addr0000"),
        amount: Uint256::from(10u64),
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "liquidate_collateral"),
            log("liquidator", "liquidator"),
            log("borrower", "addr0000"),
            log("amount", "10"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("bluna"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: HumanAddr::from("liquidation"),
                amount: Uint128::from(10u128),
                msg: Some(
                    to_binary(&LiquidationCw20HookMsg::ExecuteBid {
                        liquidator: HumanAddr::from("liquidator"),
                        fee_address: Some(HumanAddr::from("overseer")),
                        repay_address: Some(HumanAddr::from("market")),
                    })
                    .unwrap()
                ),
            })
            .unwrap(),
        })]
    );
}
