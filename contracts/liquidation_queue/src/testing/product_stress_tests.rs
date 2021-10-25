#![allow(dead_code)]
use std::str::FromStr;

use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::mock_dependencies;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi};
use cosmwasm_std::{from_binary, to_binary, Coin, Decimal, MemoryStorage, OwnedDeps, Uint128};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{
    BidsResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};

use super::mock_querier::WasmMockQuerier;

const TOLERANCE: &str = "0.00001"; // 0.001%
const ITERATIONS: u32 = 100u32;

//#[test]
fn stress_tests() {
    // submit bids and execute liquidations repeatedly
    // we can alternate larger and smaller executions to decrease the bid_pool product at different rates

    // with very tight liquidations, constatly resetting product
    // 1M UST bids
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(2000),
        1000000000000u128,
        49999999999,
        49999999990,
    );
    // 10 UST bids
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(2000),
        10000000u128,
        499999,
        499999,
    );

    // with greater asset price (10k UST per collateral)
    // 1M UST bids
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(1000000),
        1000000000000u128,
        99999999,
        99999999,
    );
    // 10,001 UST bids
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(1000000),
        10001000000u128,
        1000000,
        1000000,
    );

    // alternate tight executions, to simulate some bids claiming from 2 scales
    // 1M UST bids
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(5000),
        1000000000000u128,
        19999999999,
        19900000000,
    );
    // 100 UST bids
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(5000),
        100000000u128,
        1999999,
        1900000,
    );

    // 100k UST bids with very tight liquidations
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(10000),
        100000000000u128,
        999999999,
        999999999,
    );

    // 100k UST bids with very small asset price, so even tighter liquidations
    simulate_bids_with_2_liq_amounts(
        ITERATIONS,
        Decimal256::percent(10), // 0.1 UST/asset
        100000000000u128,
        999999999900, // 10 micros of residue
        999999999999, // no residue
    );
}

fn instantiate_and_whitelist(deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>) {
    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier
        .with_collateral_max_ltv(&[(&"col0000".to_string(), &Decimal256::percent(90))]);
    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_contract: "oracle0000".to_string(),
        stable_denom: "uusd".to_string(),
        safe_ratio: Decimal256::percent(10),
        bid_fee: Decimal256::percent(1),
        liquidator_fee: Decimal256::percent(0),
        liquidation_threshold: Uint256::from(100000000u64),
        price_timeframe: 101u64,
        waiting_period: 60u64,
        overseer: "overseer0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let env = mock_env();

    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    let msg = ExecuteMsg::WhitelistCollateral {
        collateral_token: "col0000".to_string(),
        max_slot: 30u8,
        bid_threshold: Uint256::from(10000000000000u128), // to get instant activation
        premium_rate_per_slot: Decimal256::percent(1),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();
}

fn simulate_bids_with_2_liq_amounts(
    iterations: u32,
    asset_price: Decimal256,
    bid_amount: u128,
    liq_amount_1: u128,
    liq_amount_2: u128,
) {
    let mut deps = mock_dependencies(&[]);
    instantiate_and_whitelist(&mut deps);
    deps.querier.with_oracle_price(&[(
        &("col0000".to_string(), "uusd".to_string()),
        &(
            asset_price,
            mock_env().block.time.seconds(),
            mock_env().block.time.seconds(),
        ),
    )]);

    let mut total_liquidated = Uint256::zero();
    let mut total_consumed = Uint256::zero();
    for i in 0..iterations {
        // ALICE BIDS
        let msg = ExecuteMsg::SubmitBid {
            collateral_token: "col0000".to_string(),
            premium_slot: 0u8,
        };
        let info = mock_info(
            "alice0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(bid_amount),
            }],
        );
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("col0000", &[]);
        if i % 2 == 0 {
            // EXECUTE ALL EXCEPT 1uusd
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "custody0000".to_string(),
                amount: Uint128::from(liq_amount_1),
                msg: to_binary(&Cw20HookMsg::ExecuteBid {
                    liquidator: "liquidator00000".to_string(),
                    fee_address: Some("fee0000".to_string()),
                    repay_address: Some("repay0000".to_string()),
                })
                .unwrap(),
            });
            total_liquidated += Uint256::from(liq_amount_1);
            total_consumed += Uint256::from(liq_amount_1) * asset_price;
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        } else {
            // EXECUTE ALL EXCEPT 1uusd
            let info = mock_info("col0000", &[]);
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "custody0000".to_string(),
                amount: Uint128::from(liq_amount_2),
                msg: to_binary(&Cw20HookMsg::ExecuteBid {
                    liquidator: "liquidator00000".to_string(),
                    fee_address: Some("fee0000".to_string()),
                    repay_address: Some("repay0000".to_string()),
                })
                .unwrap(),
            });
            total_liquidated += Uint256::from(liq_amount_2);
            total_consumed += Uint256::from(liq_amount_2) * asset_price;
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        }
    }

    let mut queried_bids: u32 = 0u32;
    let mut total_claimed = Uint256::zero();
    let mut total_retracted = Uint256::zero();
    while queried_bids < iterations {
        let bids_res: BidsResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BidsByUser {
                    collateral_token: "col0000".to_string(),
                    bidder: "alice0000".to_string(),
                    limit: Some(30u8),
                    start_after: Some(Uint128::from(queried_bids)),
                },
            )
            .unwrap(),
        )
        .unwrap();

        for bid in bids_res.bids.iter() {
            queried_bids += 1u32;
            println!(
                "claim idx: {} - pending: {} remaining: {}",
                bid.idx, bid.pending_liquidated_collateral, bid.amount
            );
            total_claimed += bid.pending_liquidated_collateral;
            total_retracted += bid.amount;
        }
    }
    println!("total claimed:    {}", total_claimed);
    println!("total liquidated: {}", total_liquidated);
    assert!(total_claimed < total_liquidated);

    let error: Decimal256 = Decimal256::one()
        - Decimal256::from_uint256(total_claimed) / Decimal256::from_uint256(total_liquidated);
    println!("error: {}", error);
    assert!(error < Decimal256::from_str(TOLERANCE).unwrap());
}
