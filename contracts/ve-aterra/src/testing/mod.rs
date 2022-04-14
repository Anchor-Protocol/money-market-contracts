use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, SubMsg, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use moneymarket::market::Diff;
use moneymarket::ve_aterra::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};

use crate::contract::{execute, instantiate, register_ve_aterra};

const MOCK_USER: &str = "addr0000";
const ATERRA_CONTRACT: &str = "AT-usd";
const VE_ATERRA_CONTRACT: &str = "ve-AT-usd";

fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner_addr: "owner".to_string(),
        ve_aterra_code_id: 456,
        market_addr: "market".to_string(),
        aterra_contract: ATERRA_CONTRACT.to_string(),
        stable_denom: "uust".to_string(),
        target_share: Decimal256::percent(80),
        max_pos_change: Decimal256::permille(1),
        max_neg_change: Decimal256::permille(1),
        max_rate: Decimal256::percent(10),
        min_rate: Decimal256::percent(1),
        diff_multiplier: Decimal256::percent(5),
        target_transition_amount: Decimal256::permille(1),
        premium_rate: Decimal256::percent(2),
        target_transition_epoch: 0,
        end_goal_ve_share: Default::default(),
    }
}

#[test]
fn bond() {
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
                contract_addr: "AT-usd".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(10_000u128),
                })
                .unwrap()
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "ve-AT-usd".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::from(10_000u64),
                })
                .unwrap(),
                funds: vec![]
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "market".to_string(),
                msg: to_binary(&moneymarket::market::ExecuteMsg::UpdateAterraSupply {
                    diff: Diff::Neg(Uint256::from(10_000u64)),
                })
                .unwrap(),
                funds: vec![]
            }))
        ]
    );
}
