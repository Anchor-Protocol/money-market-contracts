pub mod contract;
pub mod msg;
pub mod state;
pub mod borrow;
pub mod deposit;
pub mod querier;

mod math;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
