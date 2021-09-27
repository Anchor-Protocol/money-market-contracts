use crate::deposit::compute_exchange_rate;
use crate::state::{Config, State};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{Api, Coin, Uint128};

#[test]
fn proper_compute_exchange_rate() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(2000000u128),
    }]);
    let env = mock_env();
    //setting up the required environment for the function call (inputs)
    let mock_config = Config {
        contract_addr: deps.api.addr_canonicalize(MOCK_CONTRACT_ADDR).unwrap(),
        owner_addr: deps.api.addr_canonicalize("owner").unwrap(),
        aterra_contract: deps.api.addr_canonicalize("AT-uusd").unwrap(),
        interest_model: deps.api.addr_canonicalize("interest").unwrap(),
        distribution_model: deps.api.addr_canonicalize("distribution").unwrap(),
        distributor_contract: deps.api.addr_canonicalize("distributor").unwrap(),
        collector_contract: deps.api.addr_canonicalize("collector").unwrap(),
        overseer_contract: deps.api.addr_canonicalize("overseer").unwrap(),
        stable_denom: "uusd".to_string(),
        max_borrow_factor: Decimal256::one(),
    };
    deps.querier.with_token_balances(&[(
        &"AT-uusd".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let mock_state = State {
        total_liabilities: Decimal256::from_uint256(50000u128),
        total_reserves: Decimal256::from_uint256(550000u128),
        last_interest_updated: env.block.height,
        last_reward_updated: env.block.height,
        global_interest_index: Decimal256::one(),
        global_reward_index: Decimal256::zero(),
        anc_emission_rate: Decimal256::one(),
        prev_aterra_supply: Uint256::zero(),
        prev_exchange_rate: Decimal256::one(),
    };
    let mock_deposit_amount = Some(Uint256::from(1000000u128));

    let exchange_rate = compute_exchange_rate(
        deps.as_ref(),
        &mock_config,
        &mock_state,
        mock_deposit_amount,
    )
    .unwrap();
    assert_eq!(exchange_rate, Decimal256::percent(50));
}
