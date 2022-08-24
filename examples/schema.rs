use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use sg_p2p_nft_trade::msg::{
    ExecuteMsg, InstantiateMsg, OfferResponse, OffersResponse, ParamsResponse, QueryMsg,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(OfferResponse), &out_dir);
    export_schema(&schema_for!(OffersResponse), &out_dir);
    export_schema(&schema_for!(ParamsResponse), &out_dir);
}
