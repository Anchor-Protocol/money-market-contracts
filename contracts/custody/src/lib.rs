pub mod contract;
pub mod msg;
pub mod state;
pub mod distribution;
pub mod collateral;
pub mod external;

#[cfg(test)]
mod testing;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
