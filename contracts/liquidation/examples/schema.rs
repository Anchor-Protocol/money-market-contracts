use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use moneymarket_liquidation::msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg, BidResponse, BidsResponse, Cw20HookMsg, LiquidationAmountResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(Cw20HookMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(BidResponse), &out_dir);
    export_schema(&schema_for!(BidsResponse), &out_dir);
    export_schema(&schema_for!(LiquidationAmountResponse), &out_dir);
}
