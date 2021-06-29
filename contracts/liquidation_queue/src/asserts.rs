use crate::state::Bid;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{Env, StdError, StdResult};

const MAX_SLOT_CAP: u8 = 50u8;

pub fn assert_activate_status(bid: &Bid, env: &Env) -> StdResult<()> {
    match bid.wait_end {
        Some(wait_end) => {
            if wait_end > env.block.time {
                return Err(StdError::generic_err(format!(
                    "Wait period expires at {}",
                    wait_end
                )));
            }
        }
        None => return Err(StdError::generic_err("Bid is already active")),
    }
    Ok(())
}

pub fn assert_withdraw_amount(
    withdraw_amount: Option<Uint256>,
    withdrawable_amount: Uint256,
) -> StdResult<Uint256> {
    let to_withdraw = if let Some(amount) = withdraw_amount {
        if amount > withdrawable_amount {
            return Err(StdError::generic_err(format!(
                "Requested amount is bigger than current withdrawable amount ({})",
                withdrawable_amount
            )));
        }
        amount
    } else {
        withdrawable_amount
    };

    Ok(to_withdraw)
}

pub fn assert_max_slot(max_slot: u8) -> StdResult<()> {
    if max_slot.gt(&MAX_SLOT_CAP) {
        return Err(StdError::generic_err("Max slot exceeds limit"));
    }
    Ok(())
}
