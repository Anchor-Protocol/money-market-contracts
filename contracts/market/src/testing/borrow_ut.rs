use crate::borrow::compute_loan;
use crate::state::{Liability, State};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::mock_env;

#[test]
fn proper_compute_loan() {
    let env = mock_env("addr0000", &[]);
    let mock_state = State {
        total_liabilities: Decimal256::from_uint256(1000000u128),
        total_reserves: Decimal256::from_uint256(0u128),
        last_interest_updated: env.block.height,
        global_interest_index: Decimal256::one(),
    };
    let mut liability1 = Liability {
        interest_index: Decimal256::one(),
        loan_amount: Uint256::zero(),
    };
    compute_loan(&mock_state, &mut liability1);
    let liability2 = Liability {
        interest_index: Decimal256::one(),
        loan_amount: Uint256::zero(),
    };
    assert_eq!(liability1, liability2);

    let mock_state2 = State {
        total_liabilities: Decimal256::from_uint256(300000u128),
        total_reserves: Decimal256::from_uint256(1000u128),
        last_interest_updated: env.block.height,
        global_interest_index: Decimal256::from_uint256(2u128),
    };
    let mut liability3 = Liability {
        interest_index: Decimal256::from_uint256(4u128),
        loan_amount: Uint256::from(80u128),
    };
    compute_loan(&mock_state2, &mut liability3);
    let liability4 = Liability {
        interest_index: Decimal256::from_uint256(2u128),
        loan_amount: Uint256::from(40u128),
    };
    assert_eq!(liability3, liability4);
}
