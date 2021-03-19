use crate::borrow::{compute_borrower_interest, compute_interest};
use crate::state::{store_state, BorrowerInfo, Config, State};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{Api, Coin, HumanAddr, Uint128};

#[test]
fn proper_compute_borrower_interest() {
    let env = mock_env("addr0000", &[]);
    let mock_state = State {
        total_liabilities: Decimal256::from_uint256(1000000u128),
        total_reserves: Decimal256::from_uint256(0u128),
        last_interest_updated: env.block.height,
        last_reward_updated: env.block.height,
        global_interest_index: Decimal256::one(),
        global_reward_index: Decimal256::zero(),
        anc_emission_rate: Decimal256::one(),
        prev_aterra_supply: Uint256::zero(),
        prev_exchange_rate: Decimal256::one(),
    };
    let mut liability1 = BorrowerInfo {
        interest_index: Decimal256::one(),
        reward_index: Decimal256::zero(),
        loan_amount: Uint256::zero(),
        pending_rewards: Decimal256::zero(),
    };
    compute_borrower_interest(&mock_state, &mut liability1);
    let liability2 = BorrowerInfo {
        interest_index: Decimal256::one(),
        reward_index: Decimal256::zero(),
        loan_amount: Uint256::zero(),
        pending_rewards: Decimal256::zero(),
    };
    assert_eq!(liability1, liability2);

    let mock_state2 = State {
        total_liabilities: Decimal256::from_uint256(300000u128),
        total_reserves: Decimal256::from_uint256(1000u128),
        last_interest_updated: env.block.height,
        last_reward_updated: env.block.height,
        global_interest_index: Decimal256::from_uint256(2u128),
        global_reward_index: Decimal256::zero(),
        anc_emission_rate: Decimal256::zero(),
        prev_aterra_supply: Uint256::zero(),
        prev_exchange_rate: Decimal256::one(),
    };
    let mut liability3 = BorrowerInfo {
        interest_index: Decimal256::from_uint256(4u128),
        reward_index: Decimal256::zero(),
        loan_amount: Uint256::from(80u128),
        pending_rewards: Decimal256::zero(),
    };
    compute_borrower_interest(&mock_state2, &mut liability3);
    let liability4 = BorrowerInfo {
        interest_index: Decimal256::from_uint256(2u128),
        reward_index: Decimal256::zero(),
        loan_amount: Uint256::from(40u128),
        pending_rewards: Decimal256::zero(),
    };
    assert_eq!(liability3, liability4);
}

#[test]
fn proper_compute_interest() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(2000000u128),
        }],
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(&HumanAddr::from("addr0000"), &Uint128::from(2000000u128))],
    )]);

    let mut env = mock_env("addr0000", &[]);

    let mock_config = Config {
        contract_addr: deps
            .api
            .canonical_address(&HumanAddr::from(MOCK_CONTRACT_ADDR))
            .unwrap(),
        owner_addr: deps
            .api
            .canonical_address(&HumanAddr::from("owner"))
            .unwrap(),
        aterra_contract: deps
            .api
            .canonical_address(&HumanAddr::from("AT-uusd"))
            .unwrap(),
        interest_model: deps
            .api
            .canonical_address(&HumanAddr::from("interest"))
            .unwrap(),
        distribution_model: deps
            .api
            .canonical_address(&HumanAddr::from("distribution"))
            .unwrap(),
        distributor_contract: deps
            .api
            .canonical_address(&HumanAddr::from("distributor"))
            .unwrap(),
        collector_contract: deps
            .api
            .canonical_address(&HumanAddr::from("collector"))
            .unwrap(),
        overseer_contract: deps
            .api
            .canonical_address(&HumanAddr::from("overseer"))
            .unwrap(),
        stable_denom: "uusd".to_string(),
        max_borrow_factor: Decimal256::one(),
    };

    deps.querier
        .with_borrow_rate(&[(&HumanAddr::from("interest"), &Decimal256::percent(1))]);

    let mut mock_state = State {
        total_liabilities: Decimal256::from_uint256(1000000u128),
        total_reserves: Decimal256::zero(),
        last_interest_updated: env.block.height,
        last_reward_updated: env.block.height,
        global_interest_index: Decimal256::one(),
        global_reward_index: Decimal256::zero(),
        anc_emission_rate: Decimal256::one(),
        prev_aterra_supply: Uint256::zero(),
        prev_exchange_rate: Decimal256::one(),
    };
    store_state(&mut deps.storage, &mock_state).unwrap();

    let mock_deposit_amount = Some(Uint256::from(1000u128));

    compute_interest(
        &deps,
        &mock_config,
        &mut mock_state,
        env.block.height,
        mock_deposit_amount,
    )
    .unwrap();
    assert_eq!(
        mock_state,
        State {
            total_liabilities: Decimal256::from_uint256(1000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        }
    );

    env.block.height += 100;

    compute_interest(
        &deps,
        &mock_config,
        &mut mock_state,
        env.block.height,
        mock_deposit_amount,
    )
    .unwrap();
    assert_eq!(
        mock_state,
        State {
            total_liabilities: Decimal256::from_uint256(2000000u128),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height - 100,
            global_interest_index: Decimal256::from_uint256(2u128),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(2000000u64),
            prev_exchange_rate: Decimal256::from_ratio(14995, 10000),
        }
    );

    // exceed target_deposit_rate = 0.01
    let mut mock_state = State {
        total_liabilities: Decimal256::zero(),
        total_reserves: Decimal256::zero(),
        last_interest_updated: env.block.height,
        last_reward_updated: env.block.height,
        global_interest_index: Decimal256::one(),
        global_reward_index: Decimal256::zero(),
        anc_emission_rate: Decimal256::one(),
        prev_aterra_supply: Uint256::from(2000000u128),
        prev_exchange_rate: Decimal256::one(),
    };
    store_state(&mut deps.storage, &mock_state).unwrap();

    deps.querier.update_balance(
        MOCK_CONTRACT_ADDR,
        vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(6000000u128),
        }],
    );

    env.block.height += 100;

    // deposit_rate: 0.02
    // target_deposit_rate: 0.01
    compute_interest(&deps, &mock_config, &mut mock_state, env.block.height, None).unwrap();
    assert_eq!(
        mock_state,
        State {
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::from_uint256(2000000u64),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height - 100,
            global_interest_index: Decimal256::from_uint256(2u128),
            global_reward_index: Decimal256::zero(),
            anc_emission_rate: Decimal256::one(),
            prev_aterra_supply: Uint256::from(2000000u64),
            prev_exchange_rate: Decimal256::from_uint256(3u64),
        }
    );
}
