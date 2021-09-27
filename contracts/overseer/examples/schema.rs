use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use moneymarket::overseer::{
    AllCollateralsResponse, BorrowLimitResponse, CollateralsResponse, ConfigResponse, ExecuteMsg,
    InstantiateMsg, QueryMsg, WhitelistResponse,
};
use moneymarket_overseer::state::EpochState;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(AllCollateralsResponse), &out_dir);
    export_schema(&schema_for!(BorrowLimitResponse), &out_dir);
    export_schema(&schema_for!(CollateralsResponse), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(EpochState), &out_dir);
    export_schema(&schema_for!(WhitelistResponse), &out_dir);
}
