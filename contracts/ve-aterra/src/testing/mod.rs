use std::str::FromStr;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{to_binary, Addr, Api, CosmosMsg, SubMsg, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use moneymarket::market::Diff;
use moneymarket::ve_aterra::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};

use crate::bonding::UNBOND_DURATION_SECS;
use crate::contract::{execute, instantiate, register_ve_aterra};
use crate::state::{
    read_config, read_state, read_user_receipts, store_config, store_state, store_user_receipts,
    Config, Receipt, State,
};
use crate::testing::mock_querier::mock_dependencies;

mod mock_querier;

const MOCK_USER: &str = "addr0000";
const ATERRA_CONTRACT: &str = "AT-usd";
const VE_ATERRA_CONTRACT: &str = "ve-AT-usd";

fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner_addr: "owner".to_string(),
        ve_aterra_code_id: 456,
        market_addr: "market".to_string(),
        overseer_addr: "overseer".to_string(),
        aterra_contract: ATERRA_CONTRACT.to_string(),
        stable_denom: "uust".to_string(),
        target_share: Decimal256::percent(80),
        max_pos_change: Decimal256::permille(1),
        max_neg_change: Decimal256::permille(1),
        max_rate: Decimal256::from_str("1.20").unwrap(),
        min_rate: Decimal256::from_str("1.01").unwrap(),
        diff_multiplier: Decimal256::percent(5),
        initial_premium_rate: Decimal256::percent(2),
        premium_rate_epoch: 10,
    }
}

#[test]
fn bond_simple() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(MOCK_USER, &[]);
    let env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, mock_instantiate_msg()).unwrap();
    register_ve_aterra(deps.as_mut(), Addr::unchecked(VE_ATERRA_CONTRACT)).unwrap();
    {
        let mut state = read_state(deps.as_mut().storage).unwrap();
        state.prev_epoch_ve_aterra_exchange_rate = Decimal256::from_str("1.50").unwrap();
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: MOCK_USER.to_string(),
        amount: Uint128::from(10_000u64),
        msg: to_binary(&Cw20HookMsg::BondATerra {}).unwrap(),
    });
    let info = mock_info(ATERRA_CONTRACT, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ATERRA_CONTRACT.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(10_000u128),
                })
                .unwrap()
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: VE_ATERRA_CONTRACT.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: MOCK_USER.to_string(),
                    amount: Uint128::from(6_666u64),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateFromVeActions {
                    ve_aterra_supply: Uint256::from(6_666u64),
                    aterra_diff: Diff::Neg(Uint256::from(10_000u64)),
                    ve_exchange_rate: Decimal256::from_str("1.50").unwrap(),
                })
                .unwrap(),
                funds: vec![]
            }))
        ]
    );
}

#[test]
fn unbond_simple() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(MOCK_USER, &[]);
    let env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, mock_instantiate_msg()).unwrap();
    register_ve_aterra(deps.as_mut(), Addr::unchecked(VE_ATERRA_CONTRACT)).unwrap();

    // set state so there is 10_000 ve aterra
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state.ve_aterra_supply = Uint256::from(10_000u64);
        state.prev_epoch_ve_aterra_exchange_rate = Decimal256::from_str("1.50").unwrap();
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: MOCK_USER.to_string(),
        amount: Uint128::from(10_000u64),
        msg: to_binary(&Cw20HookMsg::UnbondVeATerra {}).unwrap(),
    });
    let info = mock_info(VE_ATERRA_CONTRACT, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: VE_ATERRA_CONTRACT.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(10_000u64),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ATERRA_CONTRACT.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: deps
                        .api
                        .addr_canonicalize(MOCK_CONTRACT_ADDR)
                        .unwrap()
                        .to_string(),
                    amount: Uint128::from(15_000u64), // 5% more
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateFromVeActions {
                    aterra_diff: Diff::Pos(Uint256::from(15_000u64)),
                    ve_aterra_supply: Uint256::from(0u64),
                    ve_exchange_rate: Decimal256::from_str("1.50").unwrap(),
                })
                .unwrap(),
                funds: vec![]
            }))
        ]
    );
    let infos = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(infos.0.len(), 1);
    assert_eq!(
        infos.0[0],
        Receipt {
            aterra_qty: Uint256::from(15_000u64),
            unlock_time: env.block.time.plus_seconds(UNBOND_DURATION_SECS)
        }
    );
}

#[test]
fn claim_simple() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(MOCK_USER, &[]);
    let env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, mock_instantiate_msg()).unwrap();
    register_ve_aterra(deps.as_mut(), Addr::unchecked(VE_ATERRA_CONTRACT)).unwrap();

    // create 2 receipts
    {
        let mut receipts = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
        receipts.0.push_back(Receipt {
            aterra_qty: Uint256::from(2_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        });
        receipts.0.push_back(Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        });
        receipts.0.push_back(Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.plus_seconds(10),
        });
        store_user_receipts(
            deps.as_mut().storage,
            &Addr::unchecked(MOCK_USER),
            &receipts,
        )
        .unwrap();
    }

    // claim less than full amount
    let msg = ExecuteMsg::ClaimATerra {
        amount: Some(5_000u64.into()),
    };
    let info = mock_info(MOCK_USER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ATERRA_CONTRACT.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: MOCK_USER.to_string(),
                amount: Uint128::from(5_000u64),
            })
            .unwrap(),
            funds: vec![]
        })),]
    );
    let receipts = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(receipts.0.len(), 2);
    assert_eq!(
        receipts.0[0],
        Receipt {
            aterra_qty: Uint256::from(7_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        }
    );
    assert_eq!(
        receipts.0[1],
        Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.plus_seconds(10),
        }
    );

    // claim full amount
    let msg = ExecuteMsg::ClaimATerra { amount: None };
    let info = mock_info(MOCK_USER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ATERRA_CONTRACT.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: MOCK_USER.to_string(),
                amount: Uint128::from(7_000u64),
            })
            .unwrap(),
            funds: vec![]
        })),]
    );
    let receipts = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(receipts.0.len(), 1);
    assert_eq!(
        receipts.0[0],
        Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.plus_seconds(10),
        }
    );
}

#[test]
fn rebond_simple() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(MOCK_USER, &[]);
    let env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, mock_instantiate_msg()).unwrap();
    register_ve_aterra(deps.as_mut(), Addr::unchecked(VE_ATERRA_CONTRACT)).unwrap();

    // set state so there is 10_000 ve aterra
    {
        let mut state = read_state(deps.as_mut().storage).unwrap();
        state.prev_epoch_ve_aterra_exchange_rate = Decimal256::from_str("2.0").unwrap();
        store_state(deps.as_mut().storage, &state).unwrap();

        let mut receipts = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
        receipts.0.push_back(Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        });
        receipts.0.push_back(Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        });
        receipts.0.push_back(Receipt {
            aterra_qty: Uint256::from(2_000u64),
            unlock_time: env.block.time.plus_seconds(10),
        });
        store_user_receipts(
            deps.as_mut().storage,
            &Addr::unchecked(MOCK_USER),
            &receipts,
        )
        .unwrap();
    }

    // claim less than full amount
    let msg = ExecuteMsg::RebondLockedATerra {
        amount: Some(5_000u64.into()),
    };
    let info = mock_info(MOCK_USER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ATERRA_CONTRACT.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(5_000u128),
                })
                .unwrap()
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: VE_ATERRA_CONTRACT.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: MOCK_USER.to_string(),
                    amount: Uint128::from(2_500u64),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateFromVeActions {
                    ve_aterra_supply: Uint256::from(2_500u64),
                    aterra_diff: Diff::Neg(Uint256::from(5_000u64)),
                    ve_exchange_rate: Decimal256::from_str("2.0").unwrap(),
                })
                .unwrap(),
                funds: vec![]
            }))
        ]
    );
    let receipts = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(receipts.0.len(), 2);
    assert_eq!(
        receipts.0[0],
        Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        }
    );
    assert_eq!(
        receipts.0[1],
        Receipt {
            aterra_qty: Uint256::from(7_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        }
    );

    // claim full amount
    let msg = ExecuteMsg::RebondLockedATerra { amount: None };
    let info = mock_info(MOCK_USER, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ATERRA_CONTRACT.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(17_000u128),
                })
                .unwrap()
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: VE_ATERRA_CONTRACT.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: MOCK_USER.to_string(),
                    amount: Uint128::from(8_500u64),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateFromVeActions {
                    ve_aterra_supply: Uint256::from(11_000u64),
                    aterra_diff: Diff::Neg(Uint256::from(17_000u64)),
                    ve_exchange_rate: Decimal256::from_str("2.0").unwrap(),
                })
                .unwrap(),
                funds: vec![]
            }))
        ]
    );
    let receipts = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(receipts.0.len(), 0);
}

#[test]
fn execute_epoch_operations_simple() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(MOCK_USER, &[]);
    let mut env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, mock_instantiate_msg()).unwrap();
    register_ve_aterra(deps.as_mut(), Addr::unchecked(VE_ATERRA_CONTRACT)).unwrap();

    // set initial config
    let mut config = read_config(deps.as_ref().storage).unwrap();
    config = Config {
        max_pos_change: Decimal256::from_str("0.005").unwrap(),
        max_neg_change: Decimal256::from_str("0.005").unwrap(),
        premium_rate_epoch: 10,
        max_rate: Decimal256::from_str("1.02").unwrap(),
        min_rate: Decimal256::from_str("1.0000").unwrap(),
        diff_multiplier: Decimal256::percent(2),
        ..config
    };
    store_config(deps.as_mut().storage, &config).unwrap();

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 10u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 50u32)]),
    ]);

    // CASE 1: hit max_pos_change
    let initial_premium = Decimal256::from_str("1.01").unwrap();
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium, // e.g. 0.1% per block
            target_share: Decimal256::percent(60),
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();
    assert_eq!(state.premium_rate, initial_premium + config.max_pos_change,);

    // CASE 2: hit max_neg_change
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium,
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 100u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 10u32)]),
    ]);

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();
    assert_eq!(state.premium_rate, initial_premium - config.max_neg_change,);

    // CASE 3: target == current => no change
    let initial_premium = Decimal256::one();
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium,
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 60u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 40u32)]),
    ]);

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();

    assert_eq!(state.premium_rate, initial_premium);

    // CASE 4: pos change, non max
    let initial_premium = Decimal256::one();
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium,
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 55u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 45u32)]),
    ]);

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();

    assert_eq!(
        state.premium_rate,
        initial_premium + Decimal256::percent(60 - 55) * config.diff_multiplier
    );

    // CASE 5: hit max overall rate
    let initial_premium = Decimal256::from_str("1.019").unwrap();
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium,
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 15u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 85u32)]),
    ]);

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();

    assert_eq!(state.premium_rate, config.max_rate,);

    // CASE 6: neg change, non max
    let initial_premium = Decimal256::from_str("1.01").unwrap();
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium,
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 629u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 371u32)]),
    ]);

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();

    // use bounds b/c decimal gets very long
    let expected = initial_premium - Decimal256::percent(65 - 60) * config.diff_multiplier;
    assert!(state.premium_rate < expected + Decimal256::from_str("0.0001").unwrap());
    assert!(state.premium_rate > expected - Decimal256::from_str("0.0001").unwrap());

    // CASE 7: hit min rate
    let initial_premium = Decimal256::from_str("1.00").unwrap();
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state = State {
            premium_rate: initial_premium,
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ..state
        };
        store_state(deps.as_mut().storage, &state).unwrap();
    }

    deps.querier.with_token_balances(&[
        (VE_ATERRA_CONTRACT, &[(MOCK_USER, 629u32)]),
        (ATERRA_CONTRACT, &[(MOCK_USER, 371u32)]),
    ]);

    env.block.height += config.premium_rate_epoch;
    let res = execute(
        deps.as_mut(),
        env,
        mock_info("overseer", &[]),
        ExecuteMsg::ExecuteEpochOperations {},
    )
    .unwrap();
    assert_eq!(res.messages, vec![]);
    let state = read_state(deps.as_ref().storage).unwrap();

    assert_eq!(state.premium_rate, initial_premium);
}
