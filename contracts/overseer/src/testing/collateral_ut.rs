use crate::collateral::compute_borrow_limit;
use crate::contract::{execute, instantiate};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::Api;

use moneymarket::overseer::{ExecuteMsg, InstantiateMsg};
use moneymarket::tokens::{Token, Tokens};

use std::str::FromStr;

#[test]
fn proper_compute_borrow_limit() {
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

    deps.querier.with_oracle_price(&[
        (
            &("bluna".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_uint256(1000u128),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
        (
            &("batom".to_string(), "uusd".to_string()),
            &(
                Decimal256::from_uint256(2000u128),
                env.block.time.seconds(),
                env.block.time.seconds(),
            ),
        ),
    ]);

    let mut collaterals: Tokens = vec![];
    let token1: Token = (
        deps.api.addr_canonicalize("bluna").unwrap(),
        Uint256::from(1000u128),
    );
    collaterals.push(token1);
    let token2: Token = (
        deps.api.addr_canonicalize("batom").unwrap(),
        Uint256::from(1000u128),
    );
    collaterals.push(token2);

    let res = compute_borrow_limit(deps.as_ref(), &collaterals, None).unwrap();
    let vec: Vec<Decimal256> = vec![
        Decimal256::from_uint256(1000u128),
        Decimal256::from_uint256(2000u128),
    ];

    let res2 = (Uint256::from(1800000u128), vec);
    assert_eq!(res, res2);
}
