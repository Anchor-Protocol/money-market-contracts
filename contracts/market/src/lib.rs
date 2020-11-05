pub mod borrow;
pub mod contract;
pub mod deposit;
pub mod msg;
pub mod querier;
pub mod state;

mod math;

#[cfg(test)]
mod testing;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);