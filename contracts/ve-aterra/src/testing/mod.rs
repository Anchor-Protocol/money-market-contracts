use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{to_binary, Addr, Api, CosmosMsg, SubMsg, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use moneymarket::market::Diff;
use moneymarket::ve_aterra::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};

use crate::bonding::UNBOND_DURATION_SECS;
use crate::contract::{execute, instantiate, register_ve_aterra};
use crate::state::{read_state, read_user_receipts, store_state, store_user_receipts, Receipt};

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
        max_rate: Decimal256::percent(10),
        min_rate: Decimal256::percent(1),
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

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: MOCK_USER.to_string(),
        amount: Uint128::from(10_000u64),
        msg: to_binary(&Cw20HookMsg::BondATerra {}).unwrap(),
    });
    let info = mock_info(ATERRA_CONTRACT, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

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
                    amount: Uint128::from(10_000u64),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateFromVeActions {
                    ve_aterra_supply: Uint256::from(10_000u64),
                    aterra_diff: Diff::Neg(Uint256::from(10_000u64)),
                    ve_exchange_rate: Decimal256::one(),
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
                    amount: Uint128::from(10_000u64), // 5% more
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateFromVeActions {
                    aterra_diff: Diff::Pos(Uint256::from(10_000u64)),
                    ve_aterra_supply: Uint256::from(0u64),
                    ve_exchange_rate: Decimal256::one(),
                })
                .unwrap(),
                funds: vec![]
            }))
        ]
    );
    let infos = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(infos.infos.len(), 1);
    assert_eq!(
        infos.infos[0],
        Receipt {
            aterra_qty: Uint256::from(10_000u64),
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

    // set state so there is 10_000 ve aterra
    {
        let mut state = read_state(deps.as_ref().storage).unwrap();
        state.ve_aterra_supply = Uint256::from(10_000u64);
        store_state(deps.as_mut().storage, &state).unwrap();

        let mut infos = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
        infos.infos.push_back(Receipt {
            aterra_qty: Uint256::from(2_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        });
        infos.infos.push_back(Receipt {
            aterra_qty: Uint256::from(10_000u64),
            unlock_time: env.block.time.minus_seconds(10),
        });
        store_user_receipts(deps.as_mut().storage, &Addr::unchecked(MOCK_USER), &infos).unwrap();
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
    let infos = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(infos.infos.len(), 1);
    assert_eq!(
        infos.infos.front().unwrap().clone(),
        Receipt {
            aterra_qty: Uint256::from(7_000u64),
            unlock_time: env.block.time.minus_seconds(10),
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
    let infos = read_user_receipts(deps.as_ref().storage, &Addr::unchecked(MOCK_USER));
    assert_eq!(infos.infos.len(), 0);
}
