use crate::state::Bid;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Env, StdError, StdResult};

const MAX_SLOT_CAP: u8 = 30u8;

pub fn assert_activate_status(
    bid: &Bid,
    env: &Env,
    available_bids: Uint256,
    bid_threshold: Uint256,
) -> StdResult<()> {
    match bid.wait_end {
        Some(wait_end) => {
            if available_bids < bid_threshold {
                // skip waiting period
                return Ok(());
            } else if wait_end > env.block.time.seconds() {
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

pub fn assert_fees(fees: Decimal256) -> StdResult<()> {
    if fees > Decimal256::one() {
        return Err(StdError::generic_err(
            "The sum of bid_fee and liquidator_fee can not be greater than one",
        ));
    }
    Ok(())
}

pub fn assert_max_slot_premium(max_slot: u8, premium_rate_per_slot: Decimal256) -> StdResult<()> {
    let max_slot_premium =
        premium_rate_per_slot * Decimal256::from_uint256(Uint256::from(max_slot as u128));
    if max_slot_premium >= Decimal256::one() {
        return Err(StdError::generic_err("Max slot premium rate exceeds limit"));
    }
    Ok(())
}
