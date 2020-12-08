use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use moneymarket::{
    BorrowLimitResponse, BorrowRateResponse, CustodyHandleMsg, DistributionParamsResponse,
    EpochStateResponse, LiquidationAmountResponse, LoanAmountResponse, MarketHandleMsg,
    PriceResponse, QueryMsg, Token, TokenHuman, Tokens, TokensHuman,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(BorrowLimitResponse), &out_dir);
    export_schema(&schema_for!(BorrowRateResponse), &out_dir);
    export_schema(&schema_for!(CustodyHandleMsg), &out_dir);
    export_schema(&schema_for!(DistributionParamsResponse), &out_dir);
    export_schema(&schema_for!(EpochStateResponse), &out_dir);
    export_schema(&schema_for!(LiquidationAmountResponse), &out_dir);
    export_schema(&schema_for!(LoanAmountResponse), &out_dir);
    export_schema(&schema_for!(MarketHandleMsg), &out_dir);
    export_schema(&schema_for!(PriceResponse), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Token), &out_dir);
    export_schema(&schema_for!(TokenHuman), &out_dir);
    export_schema(&schema_for!(Tokens), &out_dir);
    export_schema(&schema_for!(TokensHuman), &out_dir);
}
