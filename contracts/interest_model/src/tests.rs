use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::from_binary;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use moneymarket::interest_model::{
    BorrowRateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        base_rate: Decimal256::percent(10),
        interest_multiplier: Decimal256::percent(10),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0000", value.owner.as_str());
    assert_eq!("0.1", &value.base_rate.to_string());
    assert_eq!("0.1", &value.interest_multiplier.to_string());

    let query_msg = QueryMsg::BorrowRate {
        market_balance: Uint256::from(1000000u128),
        total_liabilities: Decimal256::from_uint256(500000u128),
        total_reserves: Decimal256::from_uint256(100000u128),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: BorrowRateResponse = from_binary(&res).unwrap();
    // utilization_ratio = 0.35714285714285714
    // borrow_rate = 0.035714285 + 0.1
    assert_eq!("0.135714285714285714", &value.rate.to_string());

    let query_msg = QueryMsg::BorrowRate {
        market_balance: Uint256::zero(),
        total_liabilities: Decimal256::zero(),
        total_reserves: Decimal256::zero(),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: BorrowRateResponse = from_binary(&res).unwrap();
    assert_eq!("0.1", &value.rate.to_string());
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        base_rate: Decimal256::percent(10),
        interest_multiplier: Decimal256::percent(10),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        base_rate: None,
        interest_multiplier: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0001", value.owner.as_str());
    assert_eq!("0.1", &value.base_rate.to_string());
    assert_eq!("0.1", &value.interest_multiplier.to_string());

    // Unauthorized err
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        base_rate: Some(Decimal256::percent(1)),
        interest_multiplier: Some(Decimal256::percent(1)),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return unauthorized error"),
    }
}
