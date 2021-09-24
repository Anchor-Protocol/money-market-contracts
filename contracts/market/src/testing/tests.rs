use crate::contract::{execute, instantiate, query, reply, INITIAL_DEPOSIT_AMOUNT};
use crate::error::ContractError;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{read_borrower_infos, read_state, store_state, State};
use crate::testing::mock_querier::mock_dependencies;

use anchor_token::distributor::ExecuteMsg as FaucetExecuteMsg;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, BankMsg, Coin, ContractResult, CosmosMsg, Decimal, Reply,
    SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use moneymarket::market::{
    BorrowerInfoResponse, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    StateResponse,
};
use moneymarket::querier::deduct_tax;
use protobuf::Message;
use std::str::FromStr;
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: 123u64,
                funds: vec![],
                label: "".to_string(),
                msg: to_binary(&TokenInstantiateMsg {
                    name: "Anchor Terra USD".to_string(),
                    symbol: "aUST".to_string(),
                    decimals: 6u8,
                    initial_balances: vec![Cw20Coin {
                        address: MOCK_CONTRACT_ADDR.to_string(),
                        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
                    }],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None,
                    }),
                })
                .unwrap(),
            }),
            1
        )]
    );

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg.clone()).unwrap();

    // Cannot register again
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap_err();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Cannot register again
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!("owner".to_string(), config_res.owner_addr);
    assert_eq!("AT-uusd".to_string(), config_res.aterra_contract);
    assert_eq!("interest".to_string(), config_res.interest_model);
    assert_eq!("distribution".to_string(), config_res.distribution_model);
    assert_eq!("distributor".to_string(), config_res.distributor_contract);
    assert_eq!("collector".to_string(), config_res.collector_contract);
    assert_eq!("overseer".to_string(), config_res.overseer_contract);
    assert_eq!("uusd".to_string(), config_res.stable_denom);
    assert_eq!(Decimal256::one(), config_res.max_borrow_factor);

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::State { block_height: None },
    )
    .unwrap();
    let state: StateResponse = from_binary(&query_res).unwrap();
    assert_eq!(Decimal256::zero(), state.total_liabilities);
    assert_eq!(Decimal256::zero(), state.total_reserves);
    assert_eq!(mock_env().block.height, state.last_interest_updated);
    assert_eq!(Decimal256::one(), state.global_interest_index);
    assert_eq!(Decimal256::one(), state.anc_emission_rate);
    assert_eq!(Uint256::zero(), state.prev_aterra_supply);
    assert_eq!(Decimal256::one(), state.prev_exchange_rate);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner_addr: Some("owner1".to_string()),
        interest_model: None,
        distribution_model: None,
        max_borrow_factor: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner1".to_string(), config_res.owner_addr);

    // update left items
    let info = mock_info("owner1", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner_addr: None,
        interest_model: Some("interest2".to_string()),
        distribution_model: Some("distribution2".to_string()),
        max_borrow_factor: Some(Decimal256::percent(100)),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner1".to_string(), config_res.owner_addr);
    assert_eq!("interest2".to_string(), config_res.interest_model);
    assert_eq!("distribution2".to_string(), config_res.distribution_model);
    assert_eq!(Decimal256::percent(100), config_res.max_borrow_factor);

    // Unauthorized err
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner_addr: None,
        interest_model: None,
        distribution_model: None,
        max_borrow_factor: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn deposit_stable_huge_amount() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Must deposit stable_denom
    let msg = ExecuteMsg::DepositStable {};
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(123u128),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    let _uusd_string = "uusd";
    match res {
        Err(ContractError::ZeroDeposit(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(55_555_555_000_000u128),
        }],
    );

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier.with_token_balances(&[(
        &"AT-uusd".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        )],
    )]);
    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 55_555_555_000_000u128),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_stable"),
            attr("depositor", "addr0000"),
            attr("mint_amount", "55555555000000"),
            attr("deposit_amount", "55555555000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "AT-uusd".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(55_555_555_000_000u128),
            })
            .unwrap(),
        }))]
    );

    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(111_111_110_000_000u128),
        }],
    );

    deps.querier.with_token_balances(&[(
        &"AT-uusd".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::from(55_555_555_000_000u128),
        )],
    )]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_stable"),
            attr("depositor", "addr0000"),
            attr("mint_amount", "55555555000000"),
            attr("deposit_amount", "55555555000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "AT-uusd".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(55_555_555_000_000u128),
            })
            .unwrap(),
        }))]
    );
}

#[test]
fn deposit_stable() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Must deposit stable_denom
    let msg = ExecuteMsg::DepositStable {};
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(123u128),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    let _uusd_string = "uusd";
    match res {
        Err(ContractError::ZeroDeposit(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // base denom, zero deposit
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::zero(),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::ZeroDeposit(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier.with_token_balances(&[(
        &"AT-uusd".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        )],
    )]);
    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 1000000u128),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    // 1- As the last place to modify the state is compute_interest, a check on the state ensures the invocation of compute_interest.
    // However, because passed_blocks = 0, interest factor & interest accrued are also 0, and thus the values do not change
    // (looking as if the function might not have been invoked at all.)
    // Thus, later, the invocation of compute interest will be tested after increasing the block height.
    assert_eq!(
        read_state(deps.as_ref().storage).unwrap(),
        State {
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::zero(),
            last_interest_updated: mock_env().block.height,
            last_reward_updated: mock_env().block.height,
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(1000000u64),
            prev_exchange_rate: Decimal256::one(),
        }
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_stable"),
            attr("depositor", "addr0000"),
            attr("mint_amount", "1000000"),
            attr("deposit_amount", "1000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "AT-uusd".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(1000000u128),
            })
            .unwrap(),
        }))]
    );

    // make exchange rate to 50%
    store_state(
        deps.as_mut().storage,
        &State {
            total_liabilities: Decimal256::from_uint256(50000u128),
            total_reserves: Decimal256::from_uint256(550000u128),
            last_interest_updated: mock_env().block.height,
            last_reward_updated: mock_env().block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::from_ratio(1u64, 2u64),
        },
    )
    .unwrap();

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit_stable"),
            attr("depositor", "addr0000"),
            attr("mint_amount", "2000000"),
            attr("deposit_amount", "1000000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "AT-uusd".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(2000000u128),
            })
            .unwrap(),
        }))]
    );

    // Case: compute_interest & compute_reward with block increment
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    let mut env = mock_env();

    store_state(
        deps.as_mut().storage,
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
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // State: global_interest_index: 1
    // balance: 1000000
    // aterra_supply: 1000000
    // total_liabilities: 100000
    // total_reserves: 550000
    // exchange_rate: 0.55
    // mint_amount: 0.55 * 1000000 = 1,818,181

    assert_eq!(
        read_state(deps.as_ref().storage).unwrap(),
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
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Deposit 1000000
    let msg = ExecuteMsg::DepositStable {};
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier.with_token_balances(&[(
        &"AT-uusd".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);
    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 1000000u128),
        }],
    );

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &"AT-uusd".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(2000000u128))],
    )]);

    // Redeem 1000000
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::RedeemStable {}).unwrap(),
    });
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("AT-uusd", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "AT-uusd".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(1000000u128),
                })
                .unwrap()
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(1000000u128),
                    }
                )
                .unwrap(),]
            }))
        ]
    );

    // make exchange rate to 50%
    store_state(
        deps.as_mut().storage,
        &State {
            total_liabilities: Decimal256::from_uint256(500000u128),
            total_reserves: Decimal256::from_uint256(100000u128),
            last_interest_updated: mock_env().block.height,
            last_reward_updated: mock_env().block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(2000000u64),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(500000u128),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    let _uusd_string = "uusd";
    println!("{:?}", res);
    match res {
        Err(ContractError::NoStableAvailable(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(600000u128),
        }],
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "AT-uusd".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(1000000u128),
                })
                .unwrap()
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(500000u128),
                    }
                )
                .unwrap(),]
            }))
        ]
    );
}

#[test]
fn borrow_stable() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let mut env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&"addr0000".to_string(), &Uint256::from(1000000u64))]);

    store_state(
        deps.as_mut().storage,
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

    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
        to: None,
    };

    env.block.height += 100;
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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
        res.attributes,
        vec![
            attr("action", "borrow_stable"),
            attr("borrower", "addr0000"),
            attr("borrow_amount", "500000")
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(500000u128),
                }
            )
            .unwrap()],
        }))]
    );

    assert_eq!(
        from_binary::<State>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::State { block_height: None }
            )
            .unwrap()
        )
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
                deps.as_ref(),
                mock_env(),
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
        deps.as_ref(),
        mock_env(),
        QueryMsg::BorrowerInfo {
            borrower: "addr0000".to_string(),
            block_height: None,
        },
    )
    .unwrap();

    let liability: BorrowerInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        liability,
        BorrowerInfoResponse {
            borrower: "addr0000".to_string(),
            interest_index: Decimal256::from_uint256(2u128),
            reward_index: Decimal256::from_str("0.0001").unwrap(),
            loan_amount: Uint256::from(500000u64),
            pending_rewards: Decimal256::zero(),
        }
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::BorrowerInfo {
            borrower: "addr0000".to_string(),
            block_height: Some(env.block.height),
        },
    )
    .unwrap();

    let borrower_info: BorrowerInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        borrower_info,
        BorrowerInfoResponse {
            borrower: "addr0000".to_string(),
            interest_index: Decimal256::from_uint256(2u128),
            reward_index: Decimal256::from_str("0.0001").unwrap(),
            loan_amount: Uint256::from(500000u64),
            pending_rewards: Decimal256::zero(),
        }
    );

    // Query to future blocks
    // interest_factor is 100%
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::BorrowerInfo {
            borrower: "addr0000".to_string(),
            block_height: Some(env.block.height + 100),
        },
    )
    .unwrap();

    let borrower_info: BorrowerInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        borrower_info,
        BorrowerInfoResponse {
            borrower: "addr0000".to_string(),
            interest_index: Decimal256::from_uint256(4u128),
            reward_index: Decimal256::from_str("0.00018").unwrap(),
            loan_amount: Uint256::from(1000000u64),
            pending_rewards: Decimal256::from_uint256(20u64),
        }
    );

    // Cannot borrow more than borrow limit
    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(500001u64),
        to: None,
    };
    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Err(ContractError::BorrowExceedsLimit(1000000)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn assert_max_borrow_factor() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::percent(1),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&"addr0000".to_string(), &Uint256::from(1000000u64))]);

    store_state(
        deps.as_mut().storage,
        &State {
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::zero(),
            last_interest_updated: mock_env().block.height,
            last_reward_updated: mock_env().block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )
    .unwrap();

    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(10000u64),
        to: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "borrow_stable"),
            attr("borrower", "addr0000"),
            attr("borrow_amount", "10000")
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

    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(1u64),
        to: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    let _uusd_string = "uusd";
    match res {
        Err(ContractError::MaxBorrowFactorReached(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn repay_stable() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let mut env = mock_env();
    let mut info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&"addr0000".to_string(), &Uint256::from(1000000u64))]);

    store_state(
        deps.as_mut().storage,
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

    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
        to: None,
    };

    env.block.height += 100;
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::RepayStable {};
    info.funds = vec![Coin {
        denom: "ukrw".to_string(),
        amount: Uint128::from(100000u128),
    }];

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    let _uusd_string = "uusd";
    match res {
        Err(ContractError::ZeroRepay(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    info.funds = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128::zero(),
    }];

    let res2 = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res2 {
        Err(ContractError::ZeroRepay(_uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 100000u128),
        }],
    );

    info.funds = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100000u128),
    }];
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay_stable"),
            attr("borrower", "addr0000"),
            attr("repay_amount", "100000"),
        ]
    );

    //Loan amount and Total liability have decreased according to the repayment
    let res_loan = read_borrower_infos(deps.as_ref(), None, None)
        .unwrap()
        .get(0)
        .unwrap()
        .loan_amount;
    assert_eq!(res_loan, Uint256::from(400000u128));
    assert_eq!(
        read_state(deps.as_ref().storage).unwrap().total_liabilities,
        Decimal256::from_uint256(2400000u128)
    );

    info.funds = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(500000u128),
    }];
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay_stable"),
            attr("borrower", "addr0000"),
            attr("repay_amount", "400000"),
        ]
    );

    //Loan amount and Total liability have decreased according to the repayment
    let res_loan = read_borrower_infos(deps.as_ref(), None, None)
        .unwrap()
        .get(0)
        .unwrap()
        .loan_amount;
    assert_eq!(res_loan, Uint256::zero());
    assert_eq!(
        read_state(deps.as_ref().storage).unwrap().total_liabilities,
        Decimal256::from_uint256(2000000u128)
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(100000u128),
                }
            )
            .unwrap()]
        }))]
    );
}

#[test]
fn repay_stable_from_liquidation() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let mut env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&"addr0000".to_string(), &Uint256::from(1000000u64))]);

    store_state(
        deps.as_mut().storage,
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

    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
        to: None,
    };

    env.block.height += 100;
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // update balance to make repay
    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(100000u128),
            },
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
            },
        ],
    );

    let msg = ExecuteMsg::RepayStableFromLiquidation {
        borrower: "addr0000".to_string(),
        prev_balance: Uint256::from(INITIAL_DEPOSIT_AMOUNT),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("overseer", &[]);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    let _uusd_string = "uusd";
    match res {
        Err(ContractError::ZeroRepay(__uusd_string)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // update balance to make repay
    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 100000u128),
        }],
    );

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay_stable"),
            attr("borrower", "addr0000"),
            attr("repay_amount", "100000"),
        ]
    );

    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT + 500000u128),
        }],
    );

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "repay_stable"),
            attr("borrower", "addr0000"),
            attr("repay_amount", "400000"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(100000u128),
                }
            )
            .unwrap()]
        }))]
    );
}

#[test]
fn claim_rewards() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let mut env = mock_env();
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&"addr0000".to_string(), &Uint256::from(1000000u64))]);

    store_state(
        deps.as_mut().storage,
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
    let msg = ExecuteMsg::ClaimRewards { to: None };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let msg = ExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(500000u64),
        to: None,
    };
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // zero block passed
    let msg = ExecuteMsg::ClaimRewards {
        to: Some("addr0001".to_string()),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 0);

    // 100 blocks passed
    env.block.height += 100;
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "distributor".to_string(),
            funds: vec![],
            msg: to_binary(&FaucetExecuteMsg::Spend {
                recipient: "addr0001".to_string(),
                amount: Uint128::from(33u128),
            })
            .unwrap(),
        }))]
    );

    let res: BorrowerInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::BorrowerInfo {
                borrower: "addr0000".to_string(),
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
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
    }]);
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: 123u64,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
        }],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Register anchor token contract
    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("AT-uusd".to_string());
    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };
    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    // Register overseer contract
    let msg = ExecuteMsg::RegisterContracts {
        overseer_contract: "overseer".to_string(),
        interest_model: "interest".to_string(),
        distribution_model: "distribution".to_string(),
        collector_contract: "collector".to_string(),
        distributor_contract: "distributor".to_string(),
    };
    let mut env = mock_env();
    let mut info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    deps.querier
        .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
    deps.querier
        .with_borrow_limit(&[(&"addr0000".to_string(), &Uint256::from(1000000u64))]);

    store_state(
        deps.as_mut().storage,
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
    let msg = ExecuteMsg::ExecuteEpochOperations {
        deposit_rate: Decimal256::one(),
        target_deposit_rate: Decimal256::one(),
        threshold_deposit_rate: Decimal256::one(),
        distributed_interest: Uint256::zero(),
    };

    // only overseer can execute this
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    info.sender = Addr::unchecked("overseer");
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "collector".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2970u128), // 1% tax
            }],
        }))]
    );

    let state = read_state(deps.as_ref().storage).unwrap();
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
        MOCK_CONTRACT_ADDR.to_string(),
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2999u128),
        }],
    );

    let mut env = mock_env();
    let info = mock_info("overseer", &[]);
    store_state(
        deps.as_mut().storage,
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
    let msg = ExecuteMsg::ExecuteEpochOperations {
        deposit_rate: Decimal256::one(),
        target_deposit_rate: Decimal256::one(),
        threshold_deposit_rate: Decimal256::one(),
        distributed_interest: Uint256::zero(),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let state = read_state(deps.as_ref().storage).unwrap();
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

//     let msg = InstantiateMsg {
//         owner_addr: "owner".to_string(),
//         stable_denom: "uusd".to_string(),
//         aterra_code_id: 123u64,
//         anc_emission_rate: Decimal256::one(),
//         max_borrow_factor: Decimal256::one(),
//     };

//     let env = mock_env(
//         "addr0000",
//         &[Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
//         }],
//     );

//     // we can just call .unwrap() to assert this was a success
//     let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
//     // Register anchor token contract
//     let msg = ExecuteMsg::RegisterATerra {};
//     let env = mock_env("AT-uusd", &[]);
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // Register overseer contract
//     let msg = ExecuteMsg::RegisterContracts {
//         overseer_contract: "overseer".to_string(),
//         interest_model: "interest".to_string(),
//         distribution_model: "distribution".to_string(),
//         collector_contract: "collector".to_string(),
//         distributor_contract: "distributor".to_string(),
//     };
//     let env = mock_env("addr0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     deps.querier
//         .with_borrow_rate(&[(&"interest".to_string(), &Decimal256::percent(1))]);
//     deps.querier.with_token_balances(&[(
//         &HumanAddr::from("AT-uusd"),
//         &[(
//             &MOCK_CONTRACT_ADDR.to_string(),
//             &Uint128::from(1000000u128),
//         )],
//     )]);
//     deps.querier.update_balance(
//         MOCK_CONTRACT_ADDR.to_string(),
//         vec![Coin {
//             denom: "uusd".to_string(),
//             amount: Uint128::from(373025692u128),
//         }],
//     );

//     let mut env = mock_env("overseer", &[]);

//     store_state(
//         deps.as_mut().storage,
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

//     let msg = ExecuteMsg::ExecuteEpochOperations {
//         deposit_rate: Decimal256::one(),
//         target_deposit_rate: Decimal256::from_str("0.000000040762727704").unwrap(),
//         threshold_deposit_rate: Decimal256::from_str("0.000000030572045778").unwrap(),
//     };

//     // only overseer can execute this
//     let _ = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
// }
