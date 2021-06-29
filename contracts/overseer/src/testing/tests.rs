use crate::contract::{handle, init, query};
use crate::querier::query_epoch_state;
use crate::state::{read_epoch_state, store_epoch_state, EpochState};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, log, to_binary, BankMsg, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128,
    WasmMsg,
};

use moneymarket::custody::HandleMsg as CustodyHandleMsg;
use moneymarket::market::HandleMsg as MarketHandleMsg;
use moneymarket::overseer::{
    AllCollateralsResponse, BorrowLimitResponse, CollateralsResponse, ConfigResponse, HandleMsg,
    InitMsg, QueryMsg, WhitelistResponse, WhitelistResponseElem,
};
use moneymarket::querier::deduct_tax;

use std::str::FromStr;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let query_res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        config_res,
        ConfigResponse {
            owner_addr: HumanAddr::from("owner"),
            oracle_contract: HumanAddr::from("oracle"),
            market_contract: HumanAddr::from("market"),
            liquidation_contract: HumanAddr::from("liquidation"),
            collector_contract: HumanAddr::from("collector"),
            stable_denom: "uusd".to_string(),
            epoch_period: 86400u64,
            threshold_deposit_rate: Decimal256::permille(3),
            target_deposit_rate: Decimal256::permille(5),
            buffer_distribution_factor: Decimal256::percent(20),
            anc_purchase_factor: Decimal256::percent(20),
            price_timeframe: 60u64,
        }
    );

    let query_res = query(&deps, QueryMsg::EpochState {}).unwrap();
    let epoch_state: EpochState = from_binary(&query_res).unwrap();
    assert_eq!(
        epoch_state,
        EpochState {
            deposit_rate: Decimal256::zero(),
            last_executed_height: env.block.height,
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
            prev_interest_buffer: Uint256::zero(),
        }
    );
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(20, &[]);

    let env = mock_env("addr0000", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    // update owner
    let env = mock_env("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: Some(HumanAddr("owner1".to_string())),
        oracle_contract: None,
        liquidation_contract: None,
        threshold_deposit_rate: None,
        target_deposit_rate: None,
        buffer_distribution_factor: None,
        anc_purchase_factor: None,
        epoch_period: None,
        price_timeframe: None,
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
        oracle_contract: Some(HumanAddr("oracle1".to_string())),
        liquidation_contract: Some(HumanAddr("liquidation1".to_string())),
        threshold_deposit_rate: Some(Decimal256::permille(1)),
        target_deposit_rate: Some(Decimal256::permille(2)),
        buffer_distribution_factor: Some(Decimal256::percent(10)),
        anc_purchase_factor: Some(Decimal256::percent(10)),
        epoch_period: Some(100000u64),
        price_timeframe: Some(120u64),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(HumanAddr::from("owner1"), config_res.owner_addr);
    assert_eq!(HumanAddr::from("oracle1"), config_res.oracle_contract);
    assert_eq!(
        HumanAddr::from("liquidation1"),
        config_res.liquidation_contract
    );
    assert_eq!(Decimal256::permille(1), config_res.threshold_deposit_rate);
    assert_eq!(Decimal256::permille(2), config_res.target_deposit_rate);
    assert_eq!(
        Decimal256::percent(10),
        config_res.buffer_distribution_factor
    );
    assert_eq!(Decimal256::percent(10), config_res.anc_purchase_factor);
    assert_eq!(100000u64, config_res.epoch_period);
    assert_eq!(120u64, config_res.price_timeframe);

    // Unauthorized err
    let env = mock_env("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner_addr: None,
        oracle_contract: None,
        liquidation_contract: None,
        threshold_deposit_rate: None,
        target_deposit_rate: None,
        buffer_distribution_factor: None,
        anc_purchase_factor: None,
        epoch_period: None,
        price_timeframe: None,
    };

    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn whitelist() {
    let mut deps = mock_dependencies(20, &[]);

    let env = mock_env("addr0000", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody"),
        max_ltv: Decimal256::percent(60),
    };

    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    };

    let env = mock_env("owner", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "register_whitelist"),
            log("name", "bluna"),
            log("symbol", "bluna"),
            log("collateral_token", "bluna"),
            log("custody_contract", "custody"),
            log("LTV", "0.6"),
        ]
    );

    let res = query(
        &deps,
        QueryMsg::Whitelist {
            collateral_token: Some(HumanAddr::from("bluna")),
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let whitelist_res: WhitelistResponse = from_binary(&res).unwrap();
    assert_eq!(
        whitelist_res,
        WhitelistResponse {
            elems: vec![WhitelistResponseElem {
                name: "bluna".to_string(),
                symbol: "bluna".to_string(),
                collateral_token: HumanAddr::from("bluna"),
                custody_contract: HumanAddr::from("custody"),
                max_ltv: Decimal256::percent(60),
            }]
        }
    );

    //Attempting to whitelist already whitelisted collaterals
    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody"),
        max_ltv: Decimal256::percent(60),
    };

    let env = mock_env("owner", &[]);
    let res = handle(&mut deps, env, msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Token is already registered as collateral")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::UpdateWhitelist {
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: Some(HumanAddr::from("custody2")),
        max_ltv: Some(Decimal256::percent(30)),
    };

    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    };

    let env = mock_env("owner", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "update_whitelist"),
            log("collateral_token", "bluna"),
            log("custody_contract", "custody2"),
            log("LTV", "0.3"),
        ]
    );

    let res = query(
        &deps,
        QueryMsg::Whitelist {
            collateral_token: Some(HumanAddr::from("bluna")),
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let whitelist_res: WhitelistResponse = from_binary(&res).unwrap();
    assert_eq!(
        whitelist_res,
        WhitelistResponse {
            elems: vec![WhitelistResponseElem {
                name: "bluna".to_string(),
                symbol: "bluna".to_string(),
                collateral_token: HumanAddr::from("bluna"),
                custody_contract: HumanAddr::from("custody2"),
                max_ltv: Decimal256::percent(30),
            }]
        }
    );
}

#[test]
fn execute_epoch_operations() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000000u128),
        }],
    );

    let mut env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::ExecuteEpochOperations {};
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "An epoch has not passed yet; last executed height: 12345"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }

    env.block.height += 86400u64;

    // If deposit_rate is bigger than threshold_deposit_rate
    deps.querier.with_epoch_state(&[(
        &HumanAddr::from("market"),
        &(Uint256::from(1000000u64), Decimal256::percent(120)),
    )]);

    // (120 / 100 - 1) / 86400
    // deposit rate = 0.000002314814814814
    // accrued_buffer = 10,000,000,000
    // anc_purchase_amount = accrued_buffer * 0.2 = 2,000,000,000
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("collector"),
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(2_000_000_000u128),
                    }
                )
                .unwrap()],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_batom"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::DistributeRewards {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_bluna"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::DistributeRewards {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                send: vec![],
                msg: to_binary(&HandleMsg::UpdateEpochState {
                    interest_buffer: Uint256::from(8_000_000_000u128),
                    distributed_interest: Uint256::zero(),
                })
                .unwrap(),
            })
        ]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "epoch_operations"),
            log("deposit_rate", "0.000002314814814814"),
            log("exchange_rate", "1.2"),
            log("aterra_supply", "1000000"),
            log("distributed_interest", "0"),
            log("anc_purchase_amount", "2000000000"),
        ]
    );

    // store epoch state for test purpose
    store_epoch_state(
        &mut deps.storage,
        &EpochState {
            last_executed_height: env.block.height,
            prev_exchange_rate: Decimal256::from_str("1.2").unwrap(),
            prev_aterra_supply: Uint256::from_str("1000000").unwrap(),
            prev_interest_buffer: Uint256::from_str("9999000000").unwrap(),
            deposit_rate: Decimal256::from_str("0.000002314814814814").unwrap(),
        },
    )
    .unwrap();

    // If deposit rate is bigger than threshold
    deps.querier.with_epoch_state(&[(
        &HumanAddr::from("market"),
        &(Uint256::from(1000000u64), Decimal256::percent(125)),
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    env.block.height += 86400u64;

    // accrued_buffer = 1,000,000
    // interest_buffer = 9,999,000,000
    // (125 / 120 - 1) / 86400
    // deposit rate = 0.000000482253086419
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("collector"),
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(200_000u128),
                    }
                )
                .unwrap()]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("market"),
                amount: vec![deduct_tax(
                    &deps,
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(53680u128),
                    }
                )
                .unwrap()]
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_batom"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::DistributeRewards {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_bluna"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::DistributeRewards {}).unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                send: vec![],
                msg: to_binary(&HandleMsg::UpdateEpochState {
                    interest_buffer: Uint256::from(9999746320u128),
                    distributed_interest: Uint256::from(53148u128),
                })
                .unwrap(),
            })
        ]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "epoch_operations"),
            log("deposit_rate", "0.000000482253086419"),
            log("exchange_rate", "1.25"),
            log("aterra_supply", "1000000"),
            log("distributed_interest", "53148"),
            log("anc_purchase_amount", "200000")
        ]
    );
}

#[test]
fn update_epoch_state() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(10000000000u128),
        }],
    );

    let env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    // only contract itself can execute update_epoch_state
    let msg = HandleMsg::UpdateEpochState {
        interest_buffer: Uint256::from(10000000000u128),
        distributed_interest: Uint256::from(1000000u128),
    };
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Assume execute epoch operation is executed
    let mut env = mock_env(MOCK_CONTRACT_ADDR, &[]);
    env.block.height += 86400u64;

    deps.querier.with_epoch_state(&[(
        &HumanAddr::from("market"),
        &(Uint256::from(1000000u64), Decimal256::percent(120)),
    )]);

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("market"),
            send: vec![],
            msg: to_binary(&MarketHandleMsg::ExecuteEpochOperations {
                deposit_rate: Decimal256::from_str("0.000002314814814814").unwrap(),
                target_deposit_rate: Decimal256::permille(5),
                threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
                distributed_interest: Uint256::from(1000000u128),
            })
            .unwrap(),
        })]
    );
    assert_eq!(
        res.log,
        vec![
            log("action", "update_epoch_state"),
            log("deposit_rate", "0.000002314814814814"),
            log("aterra_supply", "1000000"),
            log("exchange_rate", "1.2"),
            log("interest_buffer", "10000000000"),
        ]
    );

    // Deposit rate increased
    deps.querier.with_epoch_state(&[(
        &HumanAddr::from("market"),
        &(Uint256::from(1000000u64), Decimal256::percent(125)),
    )]);

    env.block.height += 86400u64;
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("market"),
            send: vec![],
            msg: to_binary(&MarketHandleMsg::ExecuteEpochOperations {
                deposit_rate: Decimal256::from_str("0.000000482253086419").unwrap(),
                target_deposit_rate: Decimal256::permille(5),
                threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
                distributed_interest: Uint256::from(1000000u128),
            })
            .unwrap(),
        })]
    );
    assert_eq!(
        res.log,
        vec![
            log("action", "update_epoch_state"),
            log("deposit_rate", "0.000000482253086419"),
            log("aterra_supply", "1000000"),
            log("exchange_rate", "1.25"),
            log("interest_buffer", "10000000000"),
        ]
    );

    let epoch_state_response =
        query_epoch_state(&deps, &HumanAddr::from("market"), env.block.height, None).unwrap();
    let epoch_state = read_epoch_state(&deps.storage).unwrap();

    // deposit rate = 0.000000482253078703
    assert_eq!(
        epoch_state,
        EpochState {
            deposit_rate: Decimal256::from_ratio(482253086419u64, 1000000000000000000u64),
            prev_aterra_supply: epoch_state_response.aterra_supply,
            prev_exchange_rate: epoch_state_response.exchange_rate,
            prev_interest_buffer: Uint256::from(10000000000u128),
            last_executed_height: env.block.height,
        }
    )
}

#[test]
fn lock_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::LockCollateral {
        collaterals: vec![
            (HumanAddr::from("bluna"), Uint256::from(1000000u64)),
            (HumanAddr::from("batom"), Uint256::from(10000000u64)),
        ],
    };
    let env = mock_env("addr0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_bluna"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::LockCollateral {
                    borrower: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1000000u64),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_batom"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::LockCollateral {
                    borrower: HumanAddr::from("addr0000"),
                    amount: Uint256::from(10000000u64),
                })
                .unwrap(),
            })
        ]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "lock_collateral"),
            log("borrower", "addr0000"),
            log("collaterals", "1000000bluna,10000000batom"),
        ]
    );

    let res = query(
        &deps,
        QueryMsg::Collaterals {
            borrower: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let collaterals_res: CollateralsResponse = from_binary(&res).unwrap();
    assert_eq!(
        collaterals_res,
        CollateralsResponse {
            borrower: HumanAddr::from("addr0000"),
            collaterals: vec![
                (HumanAddr::from("batom"), Uint256::from(10000000u64)),
                (HumanAddr::from("bluna"), Uint256::from(1000000u64)),
            ]
        }
    );

    let res = query(
        &deps,
        QueryMsg::AllCollaterals {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let all_collaterals_res: AllCollateralsResponse = from_binary(&res).unwrap();
    assert_eq!(
        all_collaterals_res,
        AllCollateralsResponse {
            all_collaterals: vec![CollateralsResponse {
                borrower: HumanAddr::from("addr0000"),
                collaterals: vec![
                    (HumanAddr::from("batom"), Uint256::from(10000000u64)),
                    (HumanAddr::from("bluna"), Uint256::from(1000000u64)),
                ]
            }]
        }
    );
}

#[test]
fn unlock_collateral() {
    let mut deps = mock_dependencies(20, &[]);

    let env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::LockCollateral {
        collaterals: vec![
            (HumanAddr::from("bluna"), Uint256::from(1000000u64)),
            (HumanAddr::from("batom"), Uint256::from(10000000u64)),
        ],
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // Failed to unlock more than locked amount
    let msg = HandleMsg::UnlockCollateral {
        collaterals: vec![
            (HumanAddr::from("bluna"), Uint256::from(1000001u64)),
            (HumanAddr::from("batom"), Uint256::from(10000001u64)),
        ],
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Unlock amount cannot exceed locked amount")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.with_oracle_price(&[
        (
            &("bluna".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(1000u64, 1u64),
                env.block.time,
                env.block.time,
            ),
        ),
        (
            &("batom".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(2000u64, 1u64),
                env.block.time,
                env.block.time,
            ),
        ),
    ]);

    // borrow_limit = 1000 * 1000000 * 0.6 + 2000 * 10000000 * 0.6
    // = 12,600,000,000 uusd
    deps.querier
        .with_loan_amount(&[(&HumanAddr::from("addr0000"), &Uint256::from(12600000000u64))]);

    // cannot unlock any tokens
    // Failed to unlock more than locked amount
    let msg = HandleMsg::UnlockCollateral {
        collaterals: vec![(HumanAddr::from("bluna"), Uint256::one())],
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(
                msg,
                "Unlock amount too high; Loan liability becomes greater than borrow limit: 12599999400"
            )
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::UnlockCollateral {
        collaterals: vec![(HumanAddr::from("batom"), Uint256::one())],
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(
                msg,
                "Unlock amount too high; Loan liability becomes greater than borrow limit: 12599998800"
            )
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // borrow_limit = 1000 * 1000000 * 0.6 + 2000 * 10000000 * 0.6
    // = 12,600,000,000 uusd
    deps.querier
        .with_loan_amount(&[(&HumanAddr::from("addr0000"), &Uint256::from(12599999400u64))]);
    let res = query(
        &deps,
        QueryMsg::BorrowLimit {
            borrower: HumanAddr::from("addr0000"),
            block_time: None,
        },
    )
    .unwrap();
    let borrow_limit_res: BorrowLimitResponse = from_binary(&res).unwrap();
    assert_eq!(borrow_limit_res.borrow_limit, Uint256::from(12600000000u64),);

    // Cannot unlock 2bluna
    let msg = HandleMsg::UnlockCollateral {
        collaterals: vec![(HumanAddr::from("bluna"), Uint256::from(2u64))],
    };
    let res = handle(&mut deps, env.clone(), msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Unlock amount too high; Loan liability becomes greater than borrow limit: 12599998800")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Can unlock 1bluna
    let msg = HandleMsg::UnlockCollateral {
        collaterals: vec![(HumanAddr::from("bluna"), Uint256::one())],
    };
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("custody_bluna"),
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::UnlockCollateral {
                borrower: HumanAddr::from("addr0000"),
                amount: Uint256::one(),
            })
            .unwrap(),
        }),]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "unlock_collateral"),
            log("borrower", "addr0000"),
            log("collaterals", "1bluna"),
        ]
    );

    //testing for unlocking more collaterals
    deps.querier
        .with_loan_amount(&[(&HumanAddr::from("addr0000"), &Uint256::from(125999900u128))]);

    let msg = HandleMsg::UnlockCollateral {
        collaterals: vec![
            (HumanAddr::from("bluna"), Uint256::from(1u128)),
            (HumanAddr::from("batom"), Uint256::from(1u128)),
        ],
    };
    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_bluna"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::UnlockCollateral {
                    borrower: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1u128),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_batom"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::UnlockCollateral {
                    borrower: HumanAddr::from("addr0000"),
                    amount: Uint256::from(1u128),
                })
                .unwrap(),
            })
        ]
    );
    assert_eq!(
        res.log,
        vec![
            log("action", "unlock_collateral"),
            log("borrower", "addr0000"),
            log("collaterals", "1bluna,1batom"),
        ]
    );
}

#[test]
fn liquidate_collateral() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier
        .with_liquidation_percent(&[(&HumanAddr::from("liquidation"), &Decimal256::percent(1))]);

    let env = mock_env("owner", &[]);
    let msg = InitMsg {
        owner_addr: HumanAddr::from("owner"),
        oracle_contract: HumanAddr::from("oracle"),
        market_contract: HumanAddr::from("market"),
        liquidation_contract: HumanAddr::from("liquidation"),
        collector_contract: HumanAddr::from("collector"),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // store whitelist elems
    let msg = HandleMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: HumanAddr::from("bluna"),
        custody_contract: HumanAddr::from("custody_bluna"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: HumanAddr::from("batom"),
        custody_contract: HumanAddr::from("custody_batom"),
        max_ltv: Decimal256::percent(60),
    };

    let _res = handle(&mut deps, env.clone(), msg);

    let msg = HandleMsg::LockCollateral {
        collaterals: vec![
            (HumanAddr::from("bluna"), Uint256::from(1000000u64)),
            (HumanAddr::from("batom"), Uint256::from(10000000u64)),
        ],
    };
    let env = mock_env("addr0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier.with_oracle_price(&[
        (
            &("bluna".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(1000u64, 1u64),
                env.block.time,
                env.block.time,
            ),
        ),
        (
            &("batom".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(2000u64, 1u64),
                env.block.time,
                env.block.time,
            ),
        ),
    ]);

    // borrow_limit = 1000 * 1000000 * 0.6 + 2000 * 10000000 * 0.6
    // = 12,600,000,000 uusd
    deps.querier
        .with_loan_amount(&[(&HumanAddr::from("addr0000"), &Uint256::from(12600000000u64))]);

    let msg = HandleMsg::LiquidateCollateral {
        borrower: HumanAddr::from("addr0000"),
    };
    let env = mock_env("addr0001", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Cannot liquidate safely collateralized loan")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier
        .with_loan_amount(&[(&HumanAddr::from("addr0000"), &Uint256::from(12600000001u64))]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_batom"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::LiquidateCollateral {
                    liquidator: HumanAddr::from("addr0001"),
                    borrower: HumanAddr::from("addr0000"),
                    amount: Uint256::from(100000u64),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("custody_bluna"),
                send: vec![],
                msg: to_binary(&CustodyHandleMsg::LiquidateCollateral {
                    liquidator: HumanAddr::from("addr0001"),
                    borrower: HumanAddr::from("addr0000"),
                    amount: Uint256::from(10000u64),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("market"),
                send: vec![],
                msg: to_binary(&MarketHandleMsg::RepayStableFromLiquidation {
                    borrower: HumanAddr::from("addr0000"),
                    prev_balance: Uint256::zero(),
                })
                .unwrap(),
            })
        ]
    );

    let res = query(
        &deps,
        QueryMsg::Collaterals {
            borrower: HumanAddr::from("addr0000"),
        },
    )
    .unwrap();
    let collaterals_res: CollateralsResponse = from_binary(&res).unwrap();
    assert_eq!(
        collaterals_res,
        CollateralsResponse {
            borrower: HumanAddr::from("addr0000"),
            collaterals: vec![
                (HumanAddr::from("batom"), Uint256::from(9900000u64)),
                (HumanAddr::from("bluna"), Uint256::from(990000u64)),
            ]
        }
    );
}
