use cosmwasm_std::{
    from_binary, log, to_binary, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128,
    WasmMsg,
};

use crate::contract::{handle, init};
use crate::msg::{
    ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, LiabilityResponse, LiabilitysResponse,
    LoanAmountResponse, QueryMsg,
};
use crate::querier::query;
use crate::state::{store_state, State};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg, MinterResponse};
use moneymarket::{deduct_tax, CustodyHandleMsg};
use terraswap::{InitHook, TokenInitMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        interest_model: HumanAddr::from("interest"),
        base_denom: "uusd".to_string(),
        reserve_factor: Decimal::permille(3),
        anchor_token_code_id: 123u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = init(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: 123u64,
            send: vec![],
            label: None,
            msg: to_binary(&TokenInitMsg {
                name: "Anchor Token for uusd".to_string(),
                symbol: "AT-uusd".to_string(),
                decimals: 6u8,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    cap: None,
                }),
                init_hook: Some(InitHook {
                    contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    msg: to_binary(&HandleMsg::RegisterAnchorToken {}).unwrap(),
                })
            })
            .unwrap(),
        })]
    );

    // Register anchor token contract
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Cannot register again
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap_err();

    // Register overseer contract
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Cannot register again
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap_err();

    let query_res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(HumanAddr::from("owner"), config_res.owner_addr);
    assert_eq!(HumanAddr::from("AT-uusd"), config_res.anchor_token);
    assert_eq!(HumanAddr::from("interest"), config_res.interest_model);
    assert_eq!(HumanAddr::from("overseer"), config_res.overseer_contract);
    assert_eq!("uusd".to_string(), config_res.base_denom);
    assert_eq!(Decimal::permille(3), config_res.reserve_factor);

    let query_res = query(&deps, QueryMsg::State {}).unwrap();
    let state: State = from_binary(&query_res).unwrap();
    assert_eq!(Decimal::zero(), state.total_liabilities);
    assert_eq!(Decimal::zero(), state.total_reserves);
    assert_eq!(env.block.height, state.last_interest_updated);
    assert_eq!(Decimal::one(), state.global_interest_index);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        interest_model: HumanAddr::from("interest"),
        base_denom: "uusd".to_string(),
        reserve_factor: Decimal::permille(3),
        anchor_token_code_id: 123u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // update owner
    let env = mock_env("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: Some(HumanAddr("owner1".to_string())),
        reserve_factor: None,
        interest_model: None,
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(HumanAddr::from("owner1"), config_res.owner_addr);

    // update left items
    let env = mock_env("owner1", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: None,
        reserve_factor: Some(Decimal::percent(1)),
        interest_model: Some(HumanAddr::from("interest2")),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(HumanAddr::from("owner1"), config_res.owner_addr);
    assert_eq!(Decimal::percent(1), config_res.reserve_factor);
    assert_eq!(HumanAddr::from("interest2"), config_res.interest_model);

    // Unauthorzied err
    let env = mock_env("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: None,
        reserve_factor: None,
        interest_model: None,
    };

    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn deposit_stable() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        interest_model: HumanAddr::from("interest"),
        base_denom: "uusd".to_string(),
        reserve_factor: Decimal::permille(3),
        anchor_token_code_id: 123u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Must deposit base_denom
    let msg = HandleMsg::DepositStable {};
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(123u128),
        }],
    );

    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Cannot deposit zero coins"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal::percent(1))]);
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(1000000u128),
        )],
    )]);

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_stable"),
            log("depositor", "addr0000"),
            log("mint_amount", "1000000"),
            log("deposit_amount", "1000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("AT-uusd"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Mint {
                recipient: HumanAddr::from("addr0000"),
                amount: Uint128::from(1000000u128),
            })
            .unwrap(),
        })]
    );

    // make exchange rate to 50%
    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal::from_ratio(50000u128, 1u128),
            total_reserves: Decimal::from_ratio(550000u128, 1u128),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal::one(),
        },
    )
    .unwrap();

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_stable"),
            log("depositor", "addr0000"),
            log("mint_amount", "2000000"),
            log("deposit_amount", "1000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("AT-uusd"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Mint {
                recipient: HumanAddr::from("addr0000"),
                amount: Uint128::from(2000000u128),
            })
            .unwrap(),
        })]
    );
}

#[test]
fn redeem_stable() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        interest_model: HumanAddr::from("interest"),
        base_denom: "uusd".to_string(),
        reserve_factor: Decimal::permille(3),
        anchor_token_code_id: 123u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Deposit 1000000
    let msg = HandleMsg::DepositStable {};
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal::percent(1))]);
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(1000000u128),
        )],
    )]);

    let _res = handle(&mut deps, env, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(2000000u128),
        )],
    )]);

    // Redeem 1000000
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from("addr0000"),
        amount: Uint128::from(1000000u128),
        msg: Some(to_binary(&Cw20HookMsg::RedeemStable {}).unwrap()),
    });
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env("AT-uusd", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("AT-uusd"),
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Burn {
                    amount: Uint128::from(1000000u128),
                })
                .unwrap()
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("addr0000"),
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(1000000u128),
                    }
                )
                .unwrap(),]
            })
        ]
    );

    // make exchange rate to 50%
    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal::from_ratio(500000u128, 1u128),
            total_reserves: Decimal::from_ratio(1500000u128, 1u128),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal::one(),
        },
    )
    .unwrap();

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("AT-uusd"),
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Burn {
                    amount: Uint128::from(1000000u128),
                })
                .unwrap()
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("addr0000"),
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(500000u128),
                    }
                )
                .unwrap(),]
            })
        ]
    );
}

#[test]
fn borrow_stable() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        interest_model: HumanAddr::from("interest"),
        base_denom: "uusd".to_string(),
        reserve_factor: Decimal::permille(3),
        anchor_token_code_id: 123u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint128::from(1000000u128))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal::from_ratio(1000000u128, 1u128),
            total_reserves: Decimal::zero(),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal::one(),
        },
    )
    .unwrap();

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint128::from(500000u128),
        to: None,
    };

    env.block.height += 100;
    let res = handle(&mut deps, env.clone(), msg).unwrap();

    // interest_factor = 1% * 100blocks = 1
    // interest_accrued = 1000000
    // global_interest_index = 2
    // total_liabilities = 2500000
    // total_reserves = 3000
    // last_interest_updated = 100
    assert_eq!(
        res.log,
        vec![
            log("action", "borrow_stable"),
            log("borrower", "addr0000"),
            log("borrow_amount", "500000")
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: HumanAddr::from("addr0000"),
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(500000u128),
                }
            )
            .unwrap()],
        }),]
    );

    let res = query(&deps, QueryMsg::State {}).unwrap();
    let state: State = from_binary(&res).unwrap();
    assert_eq!(
        state,
        State {
            total_liabilities: Decimal::from_ratio(2500000u128, 1u128),
            total_reserves: Decimal::from_ratio(3000u128, 1u128),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal::from_ratio(2u128, 1u128),
        }
    );

    let res = query(
        &deps,
        QueryMsg::Liability {
            borrower: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let liability: LiabilityResponse = from_binary(&res).unwrap();
    assert_eq!(
        liability,
        LiabilityResponse {
            borrower: HumanAddr::from("addr0000"),
            interest_index: Decimal::from_ratio(2u128, 1u128),
            loan_amount: Uint128::from(500000u128),
        }
    );

    // Cannot borrow more than borrow limit
    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint128::from(500001u128),
        to: None,
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Cannot borrow more than limit"),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn repay_stable() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        interest_model: HumanAddr::from("interest"),
        base_denom: "uusd".to_string(),
        reserve_factor: Decimal::permille(3),
        anchor_token_code_id: 123u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterAnchorToken {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterOverseer {
        overseer_contract: HumanAddr::from("overseer"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint128::from(1000000u128))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal::from_ratio(1000000u128, 1u128),
            total_reserves: Decimal::zero(),
            last_interest_updated: env.block.height,
            global_interest_index: Decimal::one(),
        },
    )
    .unwrap();

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint128::from(500000u128),
        to: None,
    };

    env.block.height += 100;
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::RepayStable {};
    env.message.sent_funds = vec![Coin {
        denom: "ukrw".to_string(),
        amount: Uint128(100000u128),
    }];

    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Cannot repay zero coins"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    env.message.sent_funds = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128(100000u128),
    }];
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "repay_stable"),
            log("borrower", "addr0000"),
            log("repay_amount", "100000"),
        ]
    );

    env.message.sent_funds = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128(500000u128),
    }];
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "repay_stable"),
            log("borrower", "addr0000"),
            log("repay_amount", "400000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
            to_address: HumanAddr::from("addr0000"),
            amount: vec![deduct_tax(
                &deps,
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(100000u128),
                }
            ).unwrap()]
        })]
    );
}
