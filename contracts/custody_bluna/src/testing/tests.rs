use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, from_binary, to_binary, Api, Attribute, BankMsg, Coin, ContractResult, CosmosMsg,
    Decimal, Reply, Response, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};

use crate::contract::{
    execute, instantiate, query, reply, CLAIM_REWARDS_OPERATION, SWAP_TO_STABLE_OPERATION,
};
use crate::error::ContractError;
use crate::external::handle::RewardContractExecuteMsg;
use crate::state::{read_borrower_info, BLunaAccruedRewardsResponse};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use moneymarket::custody::{
    BAssetInfo, BorrowerResponse, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use moneymarket::liquidation::Cw20HookMsg as LiquidationCw20HookMsg;
use terra_cosmwasm::create_swap_msg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!("owner".to_string(), config_res.owner);
    assert_eq!("bluna".to_string(), config_res.collateral_token);
    assert_eq!("overseer".to_string(), config_res.overseer_contract);
    assert_eq!("market".to_string(), config_res.market_contract);
    assert_eq!("reward".to_string(), config_res.reward_contract);
    assert_eq!("liquidation".to_string(), config_res.liquidation_contract);
    assert_eq!("uusd".to_string(), config_res.stable_denom);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner2".to_string()),
        liquidation_contract: Some("liquidation2".to_string()),
    };
    let info = mock_info("owner", &[]);
    execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!("owner2".to_string(), config_res.owner);
    assert_eq!("bluna".to_string(), config_res.collateral_token);
    assert_eq!("overseer".to_string(), config_res.overseer_contract);
    assert_eq!("market".to_string(), config_res.market_contract);
    assert_eq!("reward".to_string(), config_res.reward_contract);
    assert_eq!("liquidation2".to_string(), config_res.liquidation_contract);
    assert_eq!("uusd".to_string(), config_res.stable_denom);

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn deposit_collateral() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    // failed; cannot directly execute receive message
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    //invalid message sent
    let msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary("invalid").unwrap(),
    });
    let res2 = execute(deps.as_mut(), mock_env(), info, msg2);
    match res2 {
        Err(ContractError::MissingDepositCollateralHook {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("bluna", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "100"),
        ]
    );

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();

    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::from(100u128),
            spendable: Uint256::from(100u128),
        }
    );

    // Deposit more
    let info = mock_info("bluna", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "100"),
        ]
    );

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::from(200u128),
            spendable: Uint256::from(200u128),
        }
    );
}

#[test]
fn withdraw_collateral() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    let info = mock_info("bluna", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "100"),
        ]
    );

    let msg = ExecuteMsg::WithdrawCollateral {
        amount: Some(Uint256::from(110u64)),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Err(ContractError::WithdrawAmountExceedsSpendable(100)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::WithdrawCollateral {
        amount: Some(Uint256::from(50u64)),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "50"),
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "bluna".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(50u128),
            })
            .unwrap(),
        }))]
    );

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::from(50u64),
            spendable: Uint256::from(50u64),
        }
    );

    let msg = ExecuteMsg::WithdrawCollateral {
        amount: Some(Uint256::from(40u128)),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::from(10u128),
            spendable: Uint256::from(10u128),
        }
    );

    //withdraw with "None" amount
    let msg = ExecuteMsg::WithdrawCollateral { amount: None };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::zero(),
            spendable: Uint256::zero(),
        }
    );
}

#[test]
fn lock_collateral() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    let info = mock_info("bluna", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "100"),
        ]
    );

    let msg = ExecuteMsg::LockCollateral {
        borrower: "addr0000".to_string(),
        amount: Uint256::from(50u64),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    //locking more than spendable
    let info2 = mock_info("overseer", &[]);
    let msg2 = ExecuteMsg::LockCollateral {
        borrower: "addr0000".to_string(),
        amount: Uint256::from(200u128),
    };
    let res2 = execute(deps.as_mut(), mock_env(), info2, msg2).unwrap_err();

    assert_eq!(res2, ContractError::LockAmountExceedsSpendable(100));

    let info = mock_info("overseer", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "lock_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "50"),
        ]
    );

    //directly checking if spendable is decreased by amount
    let spend = read_borrower_info(
        &deps.storage,
        &deps.api.addr_canonicalize("addr0000").unwrap(),
    )
    .spendable;
    assert_eq!(spend, Uint256::from(50u128));

    let msg = ExecuteMsg::WithdrawCollateral {
        amount: Some(Uint256::from(51u64)),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Err(ContractError::WithdrawAmountExceedsSpendable(50)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::WithdrawCollateral {
        amount: Some(Uint256::from(50u64)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "50"),
        ]
    );

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::from(50u64),
            spendable: Uint256::from(0u64),
        }
    );

    // Unlock partial amount of collateral
    let msg = ExecuteMsg::UnlockCollateral {
        borrower: "addr0000".to_string(),
        amount: Uint256::from(30u64),
    };

    //unauthorized sender
    let info2 = mock_info("addr0000", &[]);
    let res2 = execute(deps.as_mut(), mock_env(), info2, msg.clone());
    match res2 {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    //unlocking more than allowed (which is 50 - 0 = 50)
    let msg3 = ExecuteMsg::UnlockCollateral {
        borrower: "addr0000".to_string(),
        amount: Uint256::from(230u128),
    };

    let info3 = mock_info("overseer", &[]);
    let res3 = execute(deps.as_mut(), mock_env(), info3, msg3);
    match res3 {
        Err(ContractError::UnlockAmountExceedsLocked(50)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("overseer", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unlock_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "30"),
        ]
    );

    //checking if amount is added to spendable
    let spend = read_borrower_info(
        &deps.storage,
        &deps.api.addr_canonicalize("addr0000").unwrap(),
    )
    .spendable;
    assert_eq!(spend, Uint256::from(30u128));

    let msg = ExecuteMsg::WithdrawCollateral {
        amount: Some(Uint256::from(30u64)),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "30"),
        ]
    );

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Borrower {
            address: "addr0000".to_string(),
        },
    )
    .unwrap();
    let borrower_res: BorrowerResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        borrower_res,
        BorrowerResponse {
            borrower: "addr0000".to_string(),
            balance: Uint256::from(20u64),
            spendable: Uint256::from(0u64),
        }
    );
}

#[test]
fn distribute_rewards() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(1000000u128),
    }]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::DistributeRewards {};
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.set_reward_balance(Uint128::new(10000000));
    deps.querier
        .set_accrued_rewards(BLunaAccruedRewardsResponse {
            rewards: Uint128::new(10000000),
        });

    let msg = ExecuteMsg::DistributeRewards {};
    let info = mock_info("overseer", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // Do not print logs at this step
    let empty_vector: Vec<Attribute> = Vec::new();
    assert_eq!(res.attributes, empty_vector);
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward".to_string(),
                funds: vec![],
                msg: to_binary(&RewardContractExecuteMsg::ClaimRewards { recipient: None })
                    .unwrap(),
            }),
            CLAIM_REWARDS_OPERATION
        )]
    );
}

#[test]
fn distribute_hook() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(1000000u128),
    }]);

    deps.querier.with_token_balances(&[(
        &"bluna".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000u128))],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Claimed rewards is 1000000uusd
    // mimic last swap_msg callback to execute distribute_hook
    deps.querier.set_other_balances(Uint128::new(1000000));
    let reply_msg = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "distribute_rewards"),
            attr("buffer_rewards", "1000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "overseer".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(990099u128)
            }],
        })),],
    )
}

#[test]
fn distribution_hook_zero_rewards() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "terraswap".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Claimed rewards is 1000000uusd
    // mimic last swap_msg callback to execute distribute_hook
    let reply_msg = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "distribute_rewards"),
            attr("buffer_rewards", "0"),
        ]
    );

    assert_eq!(res.messages, vec![],)
}

#[test]
fn swap_to_stable_denom() {
    let mut deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        },
        Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(20000000000u128),
        },
        Coin {
            denom: "usdr".to_string(),
            amount: Uint128::from(2000000u128),
        },
    ]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // mimic callback from distribute_rewards to execute swap_to_stable_denom
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };
    let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(create_swap_msg(
                Coin {
                    denom: "ukrw".to_string(),
                    amount: Uint128::from(20000000000u128),
                },
                "uusd".to_string(),
            )),
            SubMsg::reply_on_success(
                create_swap_msg(
                    Coin {
                        denom: "usdr".to_string(),
                        amount: Uint128::from(2000000u128),
                    },
                    "uusd".to_string(),
                ),
                SWAP_TO_STABLE_OPERATION
            ),
        ]
    );
}

#[test]
fn liquidate_collateral() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    let info = mock_info("bluna", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "100"),
        ]
    );

    let msg = ExecuteMsg::LockCollateral {
        borrower: "addr0000".to_string(),
        amount: Uint256::from(50u64),
    };
    let info = mock_info("overseer", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "lock_collateral"),
            attr("borrower", "addr0000"),
            attr("amount", "50"),
        ]
    );

    let msg = ExecuteMsg::LiquidateCollateral {
        liquidator: "addr0001".to_string(),
        borrower: "addr0000".to_string(),
        amount: Uint256::from(100u64),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("overseer", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Err(ContractError::LiquidationAmountExceedsLocked(50)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
    let msg = ExecuteMsg::LiquidateCollateral {
        liquidator: "liquidator".to_string(),
        borrower: "addr0000".to_string(),
        amount: Uint256::from(10u64),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "liquidate_collateral"),
            attr("liquidator", "liquidator"),
            attr("borrower", "addr0000"),
            attr("amount", "10"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "bluna".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "liquidation".to_string(),
                amount: Uint128::from(10u128),
                msg: to_binary(&LiquidationCw20HookMsg::ExecuteBid {
                    liquidator: "liquidator".to_string(),
                    fee_address: Some("overseer".to_string()),
                    repay_address: Some("market".to_string()),
                })
                .unwrap()
            })
            .unwrap(),
        }))]
    );
}

#[test]
fn proper_distribute_rewards_with_no_rewards() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(1000000u128),
    }]);

    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        collateral_token: "bluna".to_string(),
        overseer_contract: "overseer".to_string(),
        market_contract: "market".to_string(),
        reward_contract: "reward".to_string(),
        liquidation_contract: "liquidation".to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::DistributeRewards {};
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::DistributeRewards {};
    let info = mock_info("overseer", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // must return
    assert_eq!(res, Response::default());

    let msg = ExecuteMsg::DistributeRewards {};
    let info = mock_info("overseer", &[]);
    deps.querier
        .set_accrued_rewards(BLunaAccruedRewardsResponse {
            rewards: Uint128::new(0),
        });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // must return
    assert_eq!(res, Response::default());

    let msg = ExecuteMsg::DistributeRewards {};
    let info = mock_info("overseer", &[]);

    deps.querier
        .set_accrued_rewards(BLunaAccruedRewardsResponse {
            rewards: Uint128::new(10000000),
        });

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // Do not print logs at this step
    let empty_vector: Vec<Attribute> = Vec::new();
    assert_eq!(res.attributes, empty_vector);
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward".to_string(),
                funds: vec![],
                msg: to_binary(&RewardContractExecuteMsg::ClaimRewards { recipient: None })
                    .unwrap(),
            }),
            CLAIM_REWARDS_OPERATION
        ),]
    );
}
