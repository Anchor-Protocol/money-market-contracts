pub mod collateral;
pub mod contract;
pub mod distribution;
pub mod state;

mod external;

#[cfg(test)]
mod testing;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
