use crate::contract::instantiate;
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use moneymarket::ve_aterra::InstantiateMsg;

const MOCK_USER: &str = "addr0000";

fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner_addr: "owner".to_string(),
        ve_aterra_code_id: 456,
        market_addr: "market".to_string(),
        aterra_contract: "AT-uusd".to_string(),
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
    instantiate(deps.as_mut(), env, info, mock_instantiate_msg()).unwrap();
}
