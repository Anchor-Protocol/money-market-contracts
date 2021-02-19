use crate::deposit::compute_exchange_rate;
use crate::state::{Config, State};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{Api, Coin, HumanAddr, Uint128};

#[test]
fn proper_compute_exchange_rate() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    );
    let env = mock_env("addr0000", &[]);
    //setting up the required environment for the function call (inputs)
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
        faucet_contract: deps
            .api
            .canonical_address(&HumanAddr::from("faucet"))
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
        reserve_factor: Decimal256::permille(3),
        max_borrow_factor: Decimal256::one(),
    };
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("AT-uusd"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::from(1000000u128),
        )],
    )]);
    let mock_state = State {
        total_liabilities: Decimal256::from_uint256(50000u128),
        total_reserves: Decimal256::from_uint256(550000u128),
        last_interest_updated: env.block.height,
        last_reward_updated: env.block.height,
        global_interest_index: Decimal256::one(),
        global_reward_index: Decimal256::zero(),
        anc_emission_rate: Decimal256::one(),
    };
    let mock_deposit_amount = Some(Uint256::from(1000000u128));

    let exchange_rate =
        compute_exchange_rate(&deps, &mock_config, &mock_state, mock_deposit_amount).unwrap();
    assert_eq!(exchange_rate, Decimal256::percent(50));
}
