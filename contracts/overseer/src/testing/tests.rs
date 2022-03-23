use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::querier::query_epoch_state;
use crate::state::{
    read_epoch_state, store_dynrate_state, store_epoch_state, DynrateState, EpochState,
};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Api, BankMsg, CanonicalAddr, Coin, CosmosMsg, Decimal,
    DepsMut, SubMsg, Uint128, WasmMsg,
};
use moneymarket::custody::ExecuteMsg as CustodyExecuteMsg;
use moneymarket::market::ExecuteMsg as MarketExecuteMsg;
use moneymarket::overseer::{
    AllCollateralsResponse, BorrowLimitResponse, CollateralsResponse, ConfigResponse, ExecuteMsg,
    InstantiateMsg, QueryMsg, WhitelistResponse, WhitelistResponseElem,
};
use moneymarket::querier::deduct_tax;

use std::str::FromStr;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 8600u64,
        dyn_rate_maxchange: Decimal256::permille(5),
        dyn_rate_yr_increase_expectation: Decimal256::permille(1),
        dyn_rate_min: Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
        dyn_rate_max: Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        config_res,
        ConfigResponse {
            owner_addr: "owner".to_string(),
            oracle_contract: "oracle".to_string(),
            market_contract: "market".to_string(),
            liquidation_contract: "liquidation".to_string(),
            collector_contract: "collector".to_string(),
            stable_denom: "uusd".to_string(),
            epoch_period: 86400u64,
            threshold_deposit_rate: Decimal256::permille(3),
            target_deposit_rate: Decimal256::permille(5),
            buffer_distribution_factor: Decimal256::percent(20),
            anc_purchase_factor: Decimal256::percent(20),
            price_timeframe: 60u64,
            dyn_rate_epoch: 8600u64,
            dyn_rate_maxchange: Decimal256::permille(5),
            dyn_rate_yr_increase_expectation: Decimal256::permille(1),
            dyn_rate_min: Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
            dyn_rate_max: Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
        }
    );

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::EpochState {}).unwrap();
    let epoch_state: EpochState = from_binary(&query_res).unwrap();
    assert_eq!(
        epoch_state,
        EpochState {
            deposit_rate: Decimal256::zero(),
            last_executed_height: mock_env().block.height,
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
            prev_interest_buffer: Uint256::zero(),
        }
    );
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("addr0000", &[]);
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner_addr: Some("owner1".to_string()),
        oracle_contract: None,
        liquidation_contract: None,
        threshold_deposit_rate: None,
        target_deposit_rate: None,
        buffer_distribution_factor: None,
        anc_purchase_factor: None,
        epoch_period: None,
        price_timeframe: None,
        dyn_rate_epoch: None,
        dyn_rate_maxchange: None,
        dyn_rate_yr_increase_expectation: None,
        dyn_rate_min: None,
        dyn_rate_max: None,
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
        oracle_contract: Some("oracle1".to_string()),
        liquidation_contract: Some("liquidation1".to_string()),
        threshold_deposit_rate: Some(Decimal256::permille(1)),
        target_deposit_rate: Some(Decimal256::permille(2)),
        buffer_distribution_factor: Some(Decimal256::percent(10)),
        anc_purchase_factor: Some(Decimal256::percent(10)),
        epoch_period: Some(100000u64),
        price_timeframe: Some(120u64),
        dyn_rate_epoch: Some(8600u64),
        dyn_rate_maxchange: Some(Decimal256::permille(5)),
        dyn_rate_yr_increase_expectation: Some(Decimal256::permille(1)),
        dyn_rate_min: Some(Decimal256::from_ratio(
            1000000000000u64,
            1000000000000000000u64,
        )),
        dyn_rate_max: Some(Decimal256::from_ratio(
            1200000000000u64,
            1000000000000000000u64,
        )),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner1".to_string(), config_res.owner_addr);
    assert_eq!("oracle1".to_string(), config_res.oracle_contract);
    assert_eq!("liquidation1".to_string(), config_res.liquidation_contract);
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
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner_addr: None,
        oracle_contract: None,
        liquidation_contract: None,
        threshold_deposit_rate: None,
        target_deposit_rate: None,
        buffer_distribution_factor: None,
        anc_purchase_factor: None,
        epoch_period: None,
        price_timeframe: None,
        dyn_rate_epoch: None,
        dyn_rate_maxchange: None,
        dyn_rate_yr_increase_expectation: None,
        dyn_rate_min: None,
        dyn_rate_max: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn whitelist() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("addr0000", &[]);
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: "bluna".to_string(),
        custody_contract: "custody".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "register_whitelist"),
            attr("name", "bluna"),
            attr("symbol", "bluna"),
            attr("collateral_token", "bluna"),
            attr("custody_contract", "custody"),
            attr("LTV", "0.6"),
        ]
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Whitelist {
            collateral_token: Some("bluna".to_string()),
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
                collateral_token: "bluna".to_string(),
                custody_contract: "custody".to_string(),
                max_ltv: Decimal256::percent(60),
            }]
        }
    );

    //Attempting to whitelist already whitelisted collaterals
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: "bluna".to_string(),
        custody_contract: "custody".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match res {
        ContractError::TokenAlreadyRegistered {} => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::UpdateWhitelist {
        collateral_token: "bluna".to_string(),
        custody_contract: Some("custody2".to_string()),
        max_ltv: Some(Decimal256::percent(30)),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_whitelist"),
            attr("collateral_token", "bluna"),
            attr("custody_contract", "custody2"),
            attr("LTV", "0.3"),
        ]
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Whitelist {
            collateral_token: Some("bluna".to_string()),
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
                collateral_token: "bluna".to_string(),
                custody_contract: "custody2".to_string(),
                max_ltv: Decimal256::percent(30),
            }]
        }
    );
}

#[test]
fn execute_epoch_operations() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(10000000000u128),
    }]);

    let mut env = mock_env();
    let info = mock_info("owner", &[]);
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
        target_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let batom_collat_token = deps
        .api
        .addr_humanize(&CanonicalAddr::from(vec![
            1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
        .unwrap()
        .to_string();

    let bluna_collat_token = deps
        .api
        .addr_humanize(&CanonicalAddr::from(vec![
            1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
        .unwrap()
        .to_string();

    // store whitelist elems
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: bluna_collat_token,
        custody_contract: "custody_bluna".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    let msg = ExecuteMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: batom_collat_token,
        custody_contract: "custody_batom".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    let msg = ExecuteMsg::ExecuteEpochOperations {};
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res {
        Err(ContractError::EpochNotPassed(12345)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    env.block.height += 86400u64;

    // If deposit_rate is bigger than threshold_deposit_rate
    deps.querier.with_epoch_state(&[(
        &"market".to_string(),
        &(Uint256::from(1000000u64), Decimal256::percent(120)),
    )]);

    // (120 / 100 - 1) / 86400
    // deposit rate = 0.000002314814814814
    // accrued_buffer = 10,000,000,000
    // anc_purchase_amount = accrued_buffer * 0.2 = 2,000,000,000
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "collector".to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(2_000_000_000u128),
                    }
                )
                .unwrap()],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_batom".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::DistributeRewards {}).unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_bluna".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::DistributeRewards {}).unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateEpochState {
                    interest_buffer: Uint256::from(8_000_000_000u128),
                    distributed_interest: Uint256::zero(),
                })
                .unwrap(),
            }))
        ]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "epoch_operations"),
            attr("deposit_rate", "0.000002314814814814"),
            attr("exchange_rate", "1.2"),
            attr("aterra_supply", "1000000"),
            attr("distributed_interest", "0"),
            attr("anc_purchase_amount", "2000000000"),
        ]
    );

    // store epoch state for test purpose
    store_epoch_state(
        deps.as_mut().storage,
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
        &"market".to_string(),
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
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "collector".to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(200_000u128),
                    }
                )
                .unwrap()]
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "market".to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::from(53680u128),
                    }
                )
                .unwrap()]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_batom".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::DistributeRewards {}).unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_bluna".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::DistributeRewards {}).unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::UpdateEpochState {
                    interest_buffer: Uint256::from(9999746320u128),
                    distributed_interest: Uint256::from(53148u128),
                })
                .unwrap(),
            }))
        ]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "epoch_operations"),
            attr("deposit_rate", "0.000000482253086419"),
            attr("exchange_rate", "1.25"),
            attr("aterra_supply", "1000000"),
            attr("distributed_interest", "53148"),
            attr("anc_purchase_amount", "200000")
        ]
    );
}

#[test]
fn update_epoch_state() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(10000000000u128),
    }]);

    let info = mock_info("owner", &[]);
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // store whitelist elems
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: "bluna".to_string(),
        custody_contract: "custody_bluna".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    let msg = ExecuteMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: "batom".to_string(),
        custody_contract: "custody_batom".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    // only contract itself can execute update_epoch_state
    let msg = ExecuteMsg::UpdateEpochState {
        interest_buffer: Uint256::from(10000000000u128),
        distributed_interest: Uint256::from(1000000u128),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Assume execute epoch operation is executed
    let mut env = mock_env();
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    env.block.height += 86400u64;

    deps.querier.with_epoch_state(&[(
        &"market".to_string(),
        &(Uint256::from(1000000u64), Decimal256::percent(120)),
    )]);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "market".to_string(),
            funds: vec![],
            msg: to_binary(&MarketExecuteMsg::ExecuteEpochOperations {
                deposit_rate: Decimal256::from_str("0.000002314814814814").unwrap(),
                target_deposit_rate: Decimal256::permille(5),
                threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
                distributed_interest: Uint256::from(1000000u128),
            })
            .unwrap(),
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_epoch_state"),
            attr("deposit_rate", "0.000002314814814814"),
            attr("aterra_supply", "1000000"),
            attr("exchange_rate", "1.2"),
            attr("interest_buffer", "10000000000"),
        ]
    );

    // Deposit rate increased
    deps.querier.with_epoch_state(&[(
        &"market".to_string(),
        &(Uint256::from(1000000u64), Decimal256::percent(125)),
    )]);

    env.block.height += 86400u64;
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "market".to_string(),
            funds: vec![],
            msg: to_binary(&MarketExecuteMsg::ExecuteEpochOperations {
                deposit_rate: Decimal256::from_str("0.000000482253086419").unwrap(),
                target_deposit_rate: Decimal256::from_str("0.000001006442178229").unwrap(),
                threshold_deposit_rate: Decimal256::from_str("0.000001006442178229").unwrap(),
                distributed_interest: Uint256::from(1000000u128),
            })
            .unwrap(),
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_epoch_state"),
            attr("deposit_rate", "0.000000482253086419"),
            attr("aterra_supply", "1000000"),
            attr("exchange_rate", "1.25"),
            attr("interest_buffer", "10000000000"),
        ]
    );

    let epoch_state_response = query_epoch_state(
        deps.as_ref(),
        Addr::unchecked("market"),
        env.block.height,
        None,
    )
    .unwrap();
    let epoch_state = read_epoch_state(deps.as_ref().storage).unwrap();

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
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("owner", &[]);
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let batom_collat_token = deps
        .api
        .addr_humanize(&CanonicalAddr::from(vec![
            1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
        .unwrap()
        .to_string();

    let bluna_collat_token = deps
        .api
        .addr_humanize(&CanonicalAddr::from(vec![
            1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
        .unwrap()
        .to_string();

    // store whitelist elems
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: bluna_collat_token.clone(),
        custody_contract: "custody_bluna".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    let msg = ExecuteMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: batom_collat_token.clone(),
        custody_contract: "custody_batom".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg);

    let msg = ExecuteMsg::LockCollateral {
        collaterals: vec![
            (bluna_collat_token.clone(), Uint256::from(1000000u64)),
            (batom_collat_token.clone(), Uint256::from(10000000u64)),
        ],
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_bluna".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::LockCollateral {
                    borrower: "addr0000".to_string(),
                    amount: Uint256::from(1000000u64),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_batom".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::LockCollateral {
                    borrower: "addr0000".to_string(),
                    amount: Uint256::from(10000000u64),
                })
                .unwrap(),
            }))
        ]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "lock_collateral"),
            attr("borrower", "addr0000"),
            attr(
                "collaterals",
                format!(
                    "1000000{},10000000{}",
                    bluna_collat_token, batom_collat_token
                )
            ),
        ]
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Collaterals {
            borrower: "addr0000".to_string(),
        },
    )
    .unwrap();
    let collaterals_res: CollateralsResponse = from_binary(&res).unwrap();
    assert_eq!(
        collaterals_res,
        CollateralsResponse {
            borrower: "addr0000".to_string(),
            collaterals: vec![
                (batom_collat_token.clone(), Uint256::from(10000000u64)),
                (bluna_collat_token.clone(), Uint256::from(1000000u64)),
            ]
        }
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
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
                borrower: "addr0000".to_string(),
                collaterals: vec![
                    (batom_collat_token, Uint256::from(10000000u64)),
                    (bluna_collat_token, Uint256::from(1000000u64)),
                ]
            }]
        }
    );
}

#[test]
fn unlock_collateral() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("owner", &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // store whitelist elems
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: "bluna".to_string(),
        custody_contract: "custody_bluna".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    let msg = ExecuteMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: "batom".to_string(),
        custody_contract: "custody_batom".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), env.clone(), info, msg);

    let msg = ExecuteMsg::LockCollateral {
        collaterals: vec![
            ("bluna".to_string(), Uint256::from(1000000u64)),
            ("batom".to_string(), Uint256::from(10000000u64)),
        ],
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Failed to unlock more than locked amount
    let msg = ExecuteMsg::UnlockCollateral {
        collaterals: vec![
            ("bluna".to_string(), Uint256::from(1000001u64)),
            ("batom".to_string(), Uint256::from(10000001u64)),
        ],
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    match res {
        Err(ContractError::UnlockExceedsLocked {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.with_oracle_price(&[
        (
            &("bluna".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(1000u64, 1u64),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("batom".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(2000u64, 1u64),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    // borrow_limit = 1000 * 1000000 * 0.6 + 2000 * 10000000 * 0.6
    // = 12,600,000,000 uusd
    deps.querier
        .with_loan_amount(&[(&"addr0000".to_string(), &Uint256::from(12600000000u64))]);

    // cannot unlock any tokens
    // Failed to unlock more than locked amount
    let msg = ExecuteMsg::UnlockCollateral {
        collaterals: vec![("bluna".to_string(), Uint256::one())],
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    match res {
        Err(ContractError::UnlockTooLarge(12599999400)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::UnlockCollateral {
        collaterals: vec![("batom".to_string(), Uint256::one())],
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    match res {
        Err(ContractError::UnlockTooLarge(12599998800)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // borrow_limit = 1000 * 1000000 * 0.6 + 2000 * 10000000 * 0.6
    // = 12,600,000,000 uusd
    deps.querier
        .with_loan_amount(&[(&"addr0000".to_string(), &Uint256::from(12599999400u64))]);
    let res = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::BorrowLimit {
            borrower: "addr0000".to_string(),
            block_time: None,
        },
    )
    .unwrap();
    let borrow_limit_res: BorrowLimitResponse = from_binary(&res).unwrap();
    assert_eq!(borrow_limit_res.borrow_limit, Uint256::from(12600000000u64),);

    // Cannot unlock 2bluna
    let msg = ExecuteMsg::UnlockCollateral {
        collaterals: vec![("bluna".to_string(), Uint256::from(2u64))],
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    match res {
        Err(ContractError::UnlockTooLarge(12599998800)) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Can unlock 1bluna
    let msg = ExecuteMsg::UnlockCollateral {
        collaterals: vec![("bluna".to_string(), Uint256::one())],
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "custody_bluna".to_string(),
            funds: vec![],
            msg: to_binary(&CustodyExecuteMsg::UnlockCollateral {
                borrower: "addr0000".to_string(),
                amount: Uint256::one(),
            })
            .unwrap(),
        }))]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unlock_collateral"),
            attr("borrower", "addr0000"),
            attr("collaterals", "1bluna"),
        ]
    );

    //testing for unlocking more collaterals
    deps.querier
        .with_loan_amount(&[(&"addr0000".to_string(), &Uint256::from(125999900u128))]);

    let msg = ExecuteMsg::UnlockCollateral {
        collaterals: vec![
            ("bluna".to_string(), Uint256::from(1u128)),
            ("batom".to_string(), Uint256::from(1u128)),
        ],
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_bluna".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::UnlockCollateral {
                    borrower: "addr0000".to_string(),
                    amount: Uint256::from(1u128),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_batom".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::UnlockCollateral {
                    borrower: "addr0000".to_string(),
                    amount: Uint256::from(1u128),
                })
                .unwrap(),
            }))
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unlock_collateral"),
            attr("borrower", "addr0000"),
            attr("collaterals", "1bluna,1batom"),
        ]
    );
}

#[test]
fn liquidate_collateral() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_liquidation_percent(&[(&"liquidation".to_string(), &Decimal256::percent(1))]);

    let info = mock_info("owner", &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 86400u64,
        dyn_rate_maxchange: Decimal256::from_str("0.03").unwrap(),
        dyn_rate_yr_increase_expectation: Decimal256::from_str("0.01").unwrap(),
        dyn_rate_min: Decimal256::zero(),
        dyn_rate_max: Decimal256::one(),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let batom_collat_token = deps
        .api
        .addr_humanize(&CanonicalAddr::from(vec![
            1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
        .unwrap()
        .to_string();

    let bluna_collat_token = deps
        .api
        .addr_humanize(&CanonicalAddr::from(vec![
            1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
        .unwrap()
        .to_string();

    // store whitelist elems
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: bluna_collat_token.clone(),
        custody_contract: "custody_bluna".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg);

    let msg = ExecuteMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: batom_collat_token.clone(),
        custody_contract: "custody_batom".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), env.clone(), info, msg);

    let msg = ExecuteMsg::LockCollateral {
        collaterals: vec![
            (bluna_collat_token.clone(), Uint256::from(1000000u64)),
            (batom_collat_token.clone(), Uint256::from(10000000u64)),
        ],
    };
    let info = mock_info("addr0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    deps.querier.with_oracle_price(&[
        (
            &(bluna_collat_token.clone(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(1000u64, 1u64),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &(batom_collat_token.clone(), "uusd".to_string()),
            &(
                Decimal256::from_ratio(2000u64, 1u64),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    // borrow_limit = 1000 * 1000000 * 0.6 + 2000 * 10000000 * 0.6
    // = 12,600,000,000 uusd
    deps.querier
        .with_loan_amount(&[(&"addr0000".to_string(), &Uint256::from(12600000000u64))]);

    let msg = ExecuteMsg::LiquidateCollateral {
        borrower: "addr0000".to_string(),
    };
    let info = mock_info("addr0001", &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res {
        Err(ContractError::CannotLiquidateSafeLoan {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier
        .with_loan_amount(&[(&"addr0000".to_string(), &Uint256::from(12600000001u64))]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_batom".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::LiquidateCollateral {
                    liquidator: "addr0001".to_string(),
                    borrower: "addr0000".to_string(),
                    amount: Uint256::from(100000u64),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "custody_bluna".to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::LiquidateCollateral {
                    liquidator: "addr0001".to_string(),
                    borrower: "addr0000".to_string(),
                    amount: Uint256::from(10000u64),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                funds: vec![],
                msg: to_binary(&MarketExecuteMsg::RepayStableFromLiquidation {
                    borrower: "addr0000".to_string(),
                    prev_balance: Uint256::zero(),
                })
                .unwrap(),
            }))
        ]
    );

    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::Collaterals {
            borrower: "addr0000".to_string(),
        },
    )
    .unwrap();
    let collaterals_res: CollateralsResponse = from_binary(&res).unwrap();
    assert_eq!(
        collaterals_res,
        CollateralsResponse {
            borrower: "addr0000".to_string(),
            collaterals: vec![
                (batom_collat_token, Uint256::from(9900000u64)),
                (bluna_collat_token, Uint256::from(990000u64)),
            ]
        }
    );
}

#[test]
fn dynamic_rate_model() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(10000000000u128),
    }]);

    let info = mock_info("owner", &[]);
    let msg = InstantiateMsg {
        owner_addr: "owner".to_string(),
        oracle_contract: "oracle".to_string(),
        market_contract: "market".to_string(),
        liquidation_contract: "liquidation".to_string(),
        collector_contract: "collector".to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 8600u64,
        dyn_rate_maxchange: Decimal256::permille(5),
        dyn_rate_yr_increase_expectation: Decimal256::permille(1),
        dyn_rate_min: Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
        dyn_rate_max: Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // store whitelist elems
    let msg = ExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: "bluna".to_string(),
        custody_contract: "custody_bluna".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    let msg = ExecuteMsg::Whitelist {
        name: "batom".to_string(),
        symbol: "batom".to_string(),
        collateral_token: "batom".to_string(),
        custody_contract: "custody_batom".to_string(),
        max_ltv: Decimal256::percent(60),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    // only contract itself can execute update_epoch_state
    let msg = ExecuteMsg::UpdateEpochState {
        interest_buffer: Uint256::from(10000000000u128),
        distributed_interest: Uint256::from(1000000u128),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Assume execute epoch operation is executed
    let mut env = mock_env();
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    env.block.height += 86400u64;

    deps.querier.with_epoch_state(&[(
        &"market".to_string(),
        &(Uint256::from(1000000u64), Decimal256::percent(120)),
    )]);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "market".to_string(),
            funds: vec![],
            msg: to_binary(&MarketExecuteMsg::ExecuteEpochOperations {
                deposit_rate: Decimal256::from_str("0.000002314814814814").unwrap(),
                target_deposit_rate: Decimal256::permille(5),
                threshold_deposit_rate: Decimal256::from_ratio(1u64, 1000000u64),
                distributed_interest: Uint256::from(1000000u128),
            })
            .unwrap(),
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_epoch_state"),
            attr("deposit_rate", "0.000002314814814814"),
            attr("aterra_supply", "1000000"),
            attr("exchange_rate", "1.2"),
            attr("interest_buffer", "10000000000"),
        ]
    );

    // Deposit rate increased
    deps.querier.with_epoch_state(&[(
        &"market".to_string(),
        &(Uint256::from(1000000u64), Decimal256::percent(125)),
    )]);

    env.block.height += 86400u64;
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "market".to_string(),
            funds: vec![],
            msg: to_binary(&MarketExecuteMsg::ExecuteEpochOperations {
                deposit_rate: Decimal256::from_str("0.000000482253086419").unwrap(),
                target_deposit_rate: Decimal256::from_str("0.000001001073696371").unwrap(),
                threshold_deposit_rate: Decimal256::from_str("0.000001001073696371").unwrap(),
                distributed_interest: Uint256::from(1000000u128),
            })
            .unwrap(),
        }))]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_epoch_state"),
            attr("deposit_rate", "0.000000482253086419"),
            attr("aterra_supply", "1000000"),
            attr("exchange_rate", "1.25"),
            attr("interest_buffer", "10000000000"),
        ]
    );

    let epoch_state_response = query_epoch_state(
        deps.as_ref(),
        Addr::unchecked("market"),
        env.block.height,
        None,
    )
    .unwrap();
    let epoch_state = read_epoch_state(deps.as_ref().storage).unwrap();

    assert_eq!(
        epoch_state,
        EpochState {
            deposit_rate: Decimal256::from_ratio(482253086419u64, 1000000000000000000u64),
            prev_aterra_supply: epoch_state_response.aterra_supply,
            prev_exchange_rate: epoch_state_response.exchange_rate,
            prev_interest_buffer: Uint256::from(10000000000u128),
            last_executed_height: env.block.height,
        }
    );

    // Case 1: YR unchanged, expected drop in rate due to dyn_rate_yr_increase_expectation
    // Rate drop: 1001073696371 - 1000858957096 = 214739275
    // 214739275 * 4656810 (bpy) = 1e15 = dyn_rate_yr_increase_expectation
    validate_deposit_rates(
        deps.as_mut(),
        Decimal256::from_ratio(1000858957096u64, 1000000000000000000u64),
    );
    // Case 2: Stillk unchanged, repeating behavior
    // Rate drop: 1000858957096 - 1000644217821 = 214739275 = dyn_rate_yr_increase_expectation
    store_dynrate_state(
        deps.as_mut().storage,
        &DynrateState {
            last_executed_height: env.block.height,
            prev_yield_reserve: Decimal256::from_str("10000000000").unwrap(),
        },
    )
    .unwrap();
    env.block.height += 86400u64;
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    validate_deposit_rates(
        deps.as_mut(),
        Decimal256::from_ratio(1000644217821u64, 1000000000000000000u64),
    );

    // ----- YR increasing dramarically, 10x
    // Rate increase: (1001717914192 - 1000644217821) * 4656810 = 5e15 = dyn_rate_maxchange
    store_dynrate_state(
        deps.as_mut().storage,
        &DynrateState {
            last_executed_height: env.block.height,
            prev_yield_reserve: Decimal256::from_str("1000000000").unwrap(),
        },
    )
    .unwrap();
    env.block.height += 86400u64;
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    validate_deposit_rates(
        deps.as_mut(),
        Decimal256::from_ratio(1001717914192u64, 1000000000000000000u64),
    );

    // ----- YR increasing just a little, rate will still drop to compensate for dyn_rate_yr_increase_expectation
    // (1001717914192 - 1001503174896) * 4656810 = 1.0000001e15
    store_dynrate_state(
        deps.as_mut().storage,
        &DynrateState {
            last_executed_height: env.block.height,
            prev_yield_reserve: Decimal256::from_str("10000000001").unwrap(),
        },
    )
    .unwrap();
    env.block.height += 86400u64;
    let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    validate_deposit_rates(
        deps.as_mut(),
        Decimal256::from_ratio(1001503174896u64, 1000000000000000000u64),
    );

    // lets hit lower threshold
    for _i in 1..200 {
        env.block.height += 86400u64;
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    }
    validate_deposit_rates(
        deps.as_mut(),
        Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
    );

    // lets hit upper threshold
    for _i in 1..200 {
        store_dynrate_state(
            deps.as_mut().storage,
            &DynrateState {
                last_executed_height: env.block.height,
                prev_yield_reserve: Decimal256::from_str("1000000000").unwrap(),
            },
        )
        .unwrap();
        env.block.height += 86400u64;
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    }
    validate_deposit_rates(
        deps.as_mut(),
        Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
    );
}

fn validate_deposit_rates(deps: DepsMut, rate: Decimal256) {
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(
        config_res,
        ConfigResponse {
            owner_addr: "owner".to_string(),
            oracle_contract: "oracle".to_string(),
            market_contract: "market".to_string(),
            liquidation_contract: "liquidation".to_string(),
            collector_contract: "collector".to_string(),
            stable_denom: "uusd".to_string(),
            epoch_period: 86400u64,
            threshold_deposit_rate: rate,
            target_deposit_rate: rate,
            buffer_distribution_factor: Decimal256::percent(20),
            anc_purchase_factor: Decimal256::percent(20),
            price_timeframe: 60u64,
            dyn_rate_epoch: 8600u64,
            dyn_rate_maxchange: Decimal256::permille(5),
            dyn_rate_yr_increase_expectation: Decimal256::permille(1),
            dyn_rate_min: Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
            dyn_rate_max: Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
        }
    );
}
