mod asserts;
mod bid;
pub mod contract;
mod querier;
mod query;
mod state;

#[cfg(test)]
mod testing;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
