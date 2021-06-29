use crate::contract::{handle, init, query, INITIAL_DEPOSIT_AMOUNT};
use crate::state::{read_borrower_infos, read_state, store_state, State};
use crate::testing::mock_querier::mock_dependencies;

use anchor_token::distributor::HandleMsg as FaucetHandleMsg;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, log, to_binary, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128,
    WasmMsg,
};
use cw20::{Cw20CoinHuman, Cw20HandleMsg, Cw20ReceiveMsg, MinterResponse};
use moneymarket::market::{
    BorrowerInfoResponse, ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg, StateResponse,
};
use moneymarket::querier::deduct_tax;
use std::str::FromStr;
use terraswap::hook::InitHook;
use terraswap::token::InitMsg as TokenInitMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let res = init(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: 123u64,
            send: vec![],
            label: None,
            msg: to_binary(&TokenInitMsg {
                name: "Anchor Terra USD".to_string(),
                symbol: "aUST".to_string(),
                decimals: 6u8,
                initial_balances: vec![Cw20CoinHuman {
                    address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
                }],
                mint: Some(MinterResponse {
                    minter: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    cap: None,
                }),
                init_hook: Some(InitHook {
                    contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    msg: to_binary(&HandleMsg::RegisterATerra {}).unwrap(),
                })
            })
            .unwrap(),
        })]
    );

    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Cannot register again
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap_err();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Cannot register again
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap_err();

    let query_res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(HumanAddr::from("owner"), config_res.owner_addr);
    assert_eq!(HumanAddr::from("AT-uusd"), config_res.aterra_contract);
    assert_eq!(HumanAddr::from("interest"), config_res.interest_model);
    assert_eq!(
        HumanAddr::from("distribution"),
        config_res.distribution_model
    );
    assert_eq!(
        HumanAddr::from("distributor"),
        config_res.distributor_contract
    );
    assert_eq!(HumanAddr::from("collector"), config_res.collector_contract);
    assert_eq!(HumanAddr::from("overseer"), config_res.overseer_contract);
    assert_eq!("uusd".to_string(), config_res.stable_denom);
    assert_eq!(Decimal256::one(), config_res.max_borrow_factor);

    let query_res = query(&deps, QueryMsg::State { block_height: None }).unwrap();
    let state: StateResponse = from_binary(&query_res).unwrap();
    assert_eq!(Decimal256::zero(), state.total_liabilities);
    assert_eq!(Decimal256::zero(), state.total_reserves);
    assert_eq!(env.block.height, state.last_interest_updated);
    assert_eq!(Decimal256::one(), state.global_interest_index);
    assert_eq!(Decimal256::one(), state.anc_emission_rate);
    assert_eq!(Uint256::zero(), state.prev_aterra_supply);
    assert_eq!(Decimal256::one(), state.prev_exchange_rate);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // update owner
    let env = mock_env("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: Some(HumanAddr("owner1".to_string())),
        interest_model: None,
        distribution_model: None,
        max_borrow_factor: None,
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
        interest_model: Some(HumanAddr::from("interest2")),
        distribution_model: Some(HumanAddr::from("distribution2")),
        max_borrow_factor: Some(Decimal256::percent(100)),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(HumanAddr::from("owner1"), config_res.owner_addr);
    assert_eq!(HumanAddr::from("interest2"), config_res.interest_model);
    assert_eq!(
        HumanAddr::from("distribution2"),
        config_res.distribution_model
    );
    assert_eq!(Decimal256::percent(100), config_res.max_borrow_factor);

    // Unauthorized err
    let env = mock_env("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: None,
        interest_model: None,
        distribution_model: None,
        max_borrow_factor: None,
    };

    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn deposit_stable_huge_amount() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Must deposit stable_denom
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
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Deposit amount must be greater than 0 uusd")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(55_555_555_000_000u128),
        }],
    );

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        )],
    )]);
    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 55_555_555_000_000u128),
        }],
    );

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_stable"),
            log("depositor", "addr0000"),
            log("mint_amount", "55555555000000"),
            log("deposit_amount", "55555555000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("AT-uusd"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Mint {
                recipient: HumanAddr::from("addr0000"),
                amount: Uint128::from(55_555_555_000_000u128),
            })
            .unwrap(),
        })]
    );

    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(111_111_110_000_000u128),
        }],
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(55_555_555_000_000u128),
        )],
    )]);

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_stable"),
            log("depositor", "addr0000"),
            log("mint_amount", "55555555000000"),
            log("deposit_amount", "55555555000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("AT-uusd"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Mint {
                recipient: HumanAddr::from("addr0000"),
                amount: Uint128::from(55_555_555_000_000u128),
            })
            .unwrap(),
        })]
    );
}

#[test]
fn deposit_stable() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Must deposit stable_denom
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
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Deposit amount must be greater than 0 uusd")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // base denom, zero deposit
    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::zero(),
        }],
    );

    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Deposit amount must be greater than 0 uusd")
        }
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
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        )],
    )]);
    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 1000000u128),
        }],
    );

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    // 1- As the last place to modify the state is compute_interest, a check on the state ensures the invocation of compute_interest.
    // However, because passed_blocks = 0, interest factor & interest accrued are also 0, and thus the values do not change
    // (looking as if the function might not have been invoked at all.)
    // Thus, later, the invocation of compute interest will be tested after increasing the block height.
    assert_eq!(
        read_state(&deps.storage).unwrap(),
        State {
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(1000000u64),
            prev_exchange_rate: Decimal256::one(),
        }
    );

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
            total_liabilities: Decimal256::from_uint256(50000u128),
            total_reserves: Decimal256::from_uint256(550000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::from_ratio(1u64, 2u64),
        },
    )
    .unwrap();

    let res = handle(&mut deps, env, msg.clone()).unwrap();
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

    // Case: compute_interest & compute_reward with block increment
    let mut env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(50000u128),
            total_reserves: Decimal256::from_uint256(550000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::from_ratio(1u64, 2u64),
        },
    )
    .unwrap();

    env.block.height += 100;
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // State: global_interest_index: 1
    // balance: 1000000
    // aterra_supply: 1000000
    // total_liabilities: 100000
    // total_reserves: 550000
    // exchange_rate: 0.55
    // mint_amount: 0.55 * 1000000 = 1,818,181

    assert_eq!(
        read_state(&deps.storage).unwrap(),
        State {
            global_interest_index: Decimal256::from_uint256(2u128),
            global_reward_index: Decimal256::from_str("0.002").unwrap(),
            total_liabilities: Decimal256::from_uint256(100000u128),
            total_reserves: Decimal256::from_uint256(550000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(INITIAL_DEPOSIT_AMOUNT + 1818181),
            prev_exchange_rate: Decimal256::from_ratio(55u64, 100u64),
        }
    );
}

#[test]
fn redeem_stable() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
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
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(1000000u128),
        )],
    )]);
    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 1000000u128),
        }],
    );

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
            total_liabilities: Decimal256::from_uint256(500000u128),
            total_reserves: Decimal256::from_uint256(100000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(2000000u64),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(500000u128),
        }],
    );

    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Not enough uusd available; borrow demand too high")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(600000u128),
        }],
    );

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
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint256::from(1000000u64))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
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
    // reward_accrued = 100
    // global_reward_index = 0.00002
    // last_rewards_updated = 100
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

    assert_eq!(
        from_binary::<State>(&query(&deps, QueryMsg::State { block_height: None }).unwrap())
            .unwrap(),
        State {
            total_liabilities: Decimal256::from_uint256(2500000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::from_uint256(2u128),
            global_reward_index: Decimal256::from_str("0.0001").unwrap(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        }
    );

    // after 1 block state
    assert_eq!(
        from_binary::<State>(
            &query(
                &deps,
                QueryMsg::State {
                    block_height: Some(env.block.height + 1u64)
                }
            )
            .unwrap()
        )
        .unwrap(),
        State {
            total_liabilities: Decimal256::from_uint256(2525000u128),
            total_reserves: Decimal256::from_uint256(0u128),
            last_interest_updated: env.block.height + 1u64,
            last_reward_updated: env.block.height + 1u64,
            global_interest_index: Decimal256::from_str("2.02").unwrap(),
            global_reward_index: Decimal256::from_str("0.0001008").unwrap(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        }
    );

    let res = query(
        &deps,
        QueryMsg::BorrowerInfo {
            borrower: HumanAddr::from("addr0000"),
            block_height: None,
        },
    )
    .unwrap();

    let liability: BorrowerInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        liability,
        BorrowerInfoResponse {
            borrower: HumanAddr::from("addr0000"),
            interest_index: Decimal256::from_uint256(2u128),
            reward_index: Decimal256::from_str("0.0001").unwrap(),
            loan_amount: Uint256::from(500000u64),
            pending_rewards: Decimal256::zero(),
        }
    );

    let res = query(
        &deps,
        QueryMsg::BorrowerInfo {
            borrower: HumanAddr::from("addr0000"),
            block_height: Some(env.block.height),
        },
    )
    .unwrap();

    let borrower_info: BorrowerInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        borrower_info,
        BorrowerInfoResponse {
            borrower: HumanAddr::from("addr0000"),
            interest_index: Decimal256::from_uint256(2u128),
            reward_index: Decimal256::from_str("0.0001").unwrap(),
            loan_amount: Uint256::from(500000u64),
            pending_rewards: Decimal256::zero(),
        }
    );

    // Query to future blocks
    // interest_factor is 100%
    let res = query(
        &deps,
        QueryMsg::BorrowerInfo {
            borrower: HumanAddr::from("addr0000"),
            block_height: Some(env.block.height + 100),
        },
    )
    .unwrap();

    let borrower_info: BorrowerInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        borrower_info,
        BorrowerInfoResponse {
            borrower: HumanAddr::from("addr0000"),
            interest_index: Decimal256::from_uint256(4u128),
            reward_index: Decimal256::from_str("0.00018").unwrap(),
            loan_amount: Uint256::from(1000000u64),
            pending_rewards: Decimal256::from_uint256(20u64),
        }
    );

    // Cannot borrow more than borrow limit
    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(500001u64),
        to: None,
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Borrow amount too high; Loan liability becomes greater than borrow limit: 1000000"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn assert_max_borrow_factor() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::percent(1),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint256::from(1000000u64))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(10000u64),
        to: None,
    };

    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "borrow_stable"),
            log("borrower", "addr0000"),
            log("borrow_amount", "10000")
        ]
    );

    // subtract borrow amount
    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR,
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT - 10000u128),
        }],
    );

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(1u64),
        to: None,
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "Exceeds uusd max borrow factor; borrow demand too high"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn repay_stable() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint256::from(1000000u64))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
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
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Repay amount must be greater than 0 uusd")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    env.message.sent_funds = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128::zero(),
    }];

    let res2 = handle(&mut deps, env.clone(), msg.clone());
    match res2 {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Repay amount must be greater than 0 uusd")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 100000u128),
        }],
    );

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

    //Loan amount and Total liability have decreased according to the repayment
    let res_loan = read_borrower_infos(&deps, None, None)
        .unwrap()
        .get(0)
        .unwrap()
        .loan_amount;
    assert_eq!(res_loan, Uint256::from(400000u128));
    assert_eq!(
        read_state(&deps.storage).unwrap().total_liabilities,
        Decimal256::from_uint256(2400000u128)
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

    //Loan amount and Total liability have decreased according to the repayment
    let res_loan = read_borrower_infos(&deps, None, None)
        .unwrap()
        .get(0)
        .unwrap()
        .loan_amount;
    assert_eq!(res_loan, Uint256::zero());
    assert_eq!(
        read_state(&deps.storage).unwrap().total_liabilities,
        Decimal256::from_uint256(2000000u128)
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
            )
            .unwrap()]
        })]
    );
}

#[test]
fn repay_stable_from_liquidation() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();
    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint256::from(1000000u64))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
        to: None,
    };

    env.block.height += 100;
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // update balance to make repay
    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128(100000u128),
            },
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
            },
        ],
    );

    let msg = HandleMsg::RepayStableFromLiquidation {
        borrower: HumanAddr::from("addr0000"),
        prev_balance: Uint256::from(INITIAL_DEPOSIT_AMOUNT),
    };

    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let height = env.block.height;
    let mut env = mock_env("overseer", &[]);
    env.block.height = height;

    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Repay amount must be greater than 0 uusd")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // update balance to make repay
    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT + 100000u128),
        }],
    );

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "repay_stable"),
            log("borrower", "addr0000"),
            log("repay_amount", "100000"),
        ]
    );

    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT + 500000u128),
        }],
    );

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
            )
            .unwrap()]
        })]
    );
}

#[test]
fn claim_rewards() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint256::from(1000000u64))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    // zero loan claim, will return empty messages
    let msg = HandleMsg::ClaimRewards { to: None };
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let msg = HandleMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
        to: None,
    };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // zero block passed
    let msg = HandleMsg::ClaimRewards {
        to: Some(HumanAddr::from("addr0001")),
    };
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 0);

    // 100 blocks passed
    env.block.height += 100;
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("distributor"),
            send: vec![],
            msg: to_binary(&FaucetHandleMsg::Spend {
                recipient: HumanAddr::from("addr0001"),
                amount: Uint128(33u128),
            })
            .unwrap(),
        })]
    );

    let res: BorrowerInfoResponse = from_binary(
        &query(
            &deps,
            QueryMsg::BorrowerInfo {
                borrower: HumanAddr::from("addr0000"),
                block_height: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res.pending_rewards,
        Decimal256::from_str("0.333333333333").unwrap()
    );
    assert_eq!(
        res.reward_index,
        Decimal256::from_str("0.000066666666666666").unwrap()
    );
}

#[test]
fn execute_epoch_operations() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let env = mock_env(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // Register anchor token contract
    let msg = HandleMsg::RegisterATerra {};
    let env = mock_env("AT-uusd", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // Register overseer contract
    let msg = HandleMsg::RegisterContracts {
        overseer_contract: HumanAddr::from("overseer"),
        interest_model: HumanAddr::from("interest"),
        distribution_model: HumanAddr::from("distribution"),
        collector_contract: HumanAddr::from("collector"),
        distributor_contract: HumanAddr::from("distributor"),
    };
    let mut env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&HumanAddr::from("addr0000"), &Uint256::from(1000000u64))]);

    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::from_uint256(3000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    env.block.height += 100;

    // reserve == 3000
    let msg = HandleMsg::ExecuteEpochOperations {
        deposit_rate: Decimal256::one(),
        target_deposit_rate: Decimal256::one(),
        threshold_deposit_rate: Decimal256::one(),
        distributed_interest: Uint256::zero(),
    };

    // only overseer can execute this
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    env.message.sender = HumanAddr::from("overseer");
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: HumanAddr::from("collector"),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2970u128), // 1% tax
            }],
        })]
    );

    let state = read_state(&deps.storage).unwrap();
    assert_eq!(
        state,
        State {
            total_liabilities: Decimal256::from_uint256(2000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::from_uint256(2u64),
            global_reward_index: Decimal256::from_str("0.0001").unwrap(),
            anc_emission_rate: Decimal256::from_uint256(5u64),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        }
    );

    // When there is not enough balance to cover reserve
    // no message will be sent and reserve will be left as same
    deps.querier.update_balance(
        HumanAddr::from(MOCK_CONTRACT_ADDR),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2999u128),
        }],
    );

    let mut env = mock_env("overseer", &[]);
    store_state(
        &mut deps.storage,
        &State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::from_uint256(3000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    env.block.height += 100;

    // reserve == 3000
    let msg = HandleMsg::ExecuteEpochOperations {
        deposit_rate: Decimal256::one(),
        target_deposit_rate: Decimal256::one(),
        threshold_deposit_rate: Decimal256::one(),
        distributed_interest: Uint256::zero(),
    };

    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let state = read_state(&deps.storage).unwrap();
    assert_eq!(
        state,
        State {
            total_liabilities: Decimal256::from_uint256(2000000u128),
            total_reserves: Decimal256::from_uint256(3000u128),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::from_uint256(2u64),
            global_reward_index: Decimal256::from_str("0.0001").unwrap(),
            anc_emission_rate: Decimal256::from_uint256(5u64),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        }
    );
}

// #[test]
// fn borrow_repay_execute_operations() {
//     let mut deps = mock_dependencies(
//         20,
//         &[Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
//         }],
//     );

//     let msg = InitMsg {
//         owner_addr: HumanAddr::from("owner"),
//         stable_denom: "uusd".to_string(),
//         aterra_code_id: 123u64,
//         anc_emission_rate: Decimal256::one(),
//         max_borrow_factor: Decimal256::one(),
//     };

//     let env = mock_env(
//         "addr0000",
//         &[Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128(INITIAL_DEPOSIT_AMOUNT),
//         }],
//     );

//     // we can just call .unwrap() to assert this was a success
//     let _res = init(&mut deps, env.clone(), msg).unwrap();
//     // Register anchor token contract
//     let msg = HandleMsg::RegisterATerra {};
//     let env = mock_env("AT-uusd", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();

//     // Register overseer contract
//     let msg = HandleMsg::RegisterContracts {
//         overseer_contract: HumanAddr::from("overseer"),
//         interest_model: HumanAddr::from("interest"),
//         distribution_model: HumanAddr::from("distribution"),
//         collector_contract: HumanAddr::from("collector"),
//         distributor_contract: HumanAddr::from("distributor"),
//     };
//     let env = mock_env("addr0000", &[]);
//     let _res = handle(&mut deps, env.clone(), msg).unwrap();

//     deps.querier
//         .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);
//     deps.querier.with_token_balances(&[(
//         &HumanAddr::from("AT-uusd"),
//         &[(
//             &HumanAddr::from(MOCK_CONTRACT_ADDR),
//             &Uint128::from(1000000u128),
//         )],
//     )]);
//     deps.querier.update_balance(
//         HumanAddr::from(MOCK_CONTRACT_ADDR),
//         vec![Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(373025692u128),
//         }],
//     );

//     let mut env = mock_env("overseer", &[]);

//     store_state(
//         &mut deps.storage,
//         &State {
//             total_liabilities: Decimal256::from_str("8.198749212085782102").unwrap(),
//             total_reserves: Decimal256::from_str("372025697.802295205294219818").unwrap(),
//             last_interest_updated: env.block.height,
//             last_reward_updated: env.block.height,
//             global_interest_index: Decimal256::from_str("1.000005078160215988").unwrap(),
//             global_reward_index: Decimal256::from_str("119531.277425251814227128").unwrap(),
//             anc_emission_rate: Decimal256::from_str("980001.99").unwrap(),
//             prev_aterra_supply: Uint256::from(1000000001000000u128),
//             prev_exchange_rate: Decimal256::from_str("1.0000022").unwrap(),
//         },
//     )
//     .unwrap();

//     env.block.height += 100;

//     let msg = HandleMsg::ExecuteEpochOperations {
//         deposit_rate: Decimal256::one(),
//         target_deposit_rate: Decimal256::from_str("0.000000040762727704").unwrap(),
//         threshold_deposit_rate: Decimal256::from_str("0.000000030572045778").unwrap(),
//     };

//     // only overseer can execute this
//     let _ = handle(&mut deps, env.clone(), msg.clone()).unwrap();
// }
