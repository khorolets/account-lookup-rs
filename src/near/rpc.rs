use borsh::BorshDeserialize;
use serde::{Deserialize};
use serde_json::json;
use crate::near::lockup_contract::{LockupContract, TransfersInformation, U64};

#[derive(Debug, Deserialize)]
pub struct Response {
    pub id: String,
    pub jsonrpc: String,
    pub result: ResponseResult,
}

#[derive(Debug, Deserialize)]
pub struct ResponseBlock {
    pub id: String,
    pub jsonrpc: String,
    pub result: Block,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
}

#[derive(Debug, Deserialize)]
pub struct ResponseResult {
    pub block_hash: String,
    pub block_height: u64,
    pub proof: Vec<String>,
    pub values: Vec<ResponseValue>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseValue {
    pub key: String,
    pub proof: Vec<String>,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
}

pub async fn get_block(block_height: Option<u64>) -> Result<BlockHeader, reqwest::Error> {
    let block_id = if let Some(block_height) = block_height {
        json!({"block_id": block_height})
    } else {
        json!({"finality": "final"})
    };
    let params = json!({
        "jsonrpc": "2.0",
        "id": "dontcare",
        "method": "block",
        "params": block_id,
    });

    let client = reqwest::Client::new();

    let res = client
        .post("https://rpc.mainnet.internal.near.org")
        .json(&params)
        .send()
        .await?;
    let body: ResponseBlock = res.json().await?;

    Ok(body.result.header)
}

pub async fn get_account_state(account_id: String, block_height: u64) -> Result<Option<LockupContract>, reqwest::Error> {
    let params = json!({
        "jsonrpc": "2.0",
        "id": "dontcare",
        "method": "query",
        "params": json!({
            "request_type": "view_state",
            "block_id": block_height,
            "account_id": account_id,
            "prefix_base64": ""
        })
    });

    let client = reqwest::Client::new();
    let res = client
        .post("https://rpc.mainnet.internal.near.org")
        .json(&params)
        .send()
        .await?;

    let body: Response = res.json().await?;

    if let Some(val) = &body.result.values.get(0) {
        let mut state = LockupContract::try_from_slice(
            base64::decode(
                &val.value
            )
                .unwrap()
                .as_slice()
        )
            .unwrap();
        // If owner of the lockup account didn't call the
        // `check_transfers_vote` contract method we won't be able to
        // get proper information based on timestamp, that's why we inject
        // the `transfer_timestamp` which is phase2 timestamp
        println!("{:?}", state.lockup_information);
        state.lockup_information.transfers_information = TransfersInformation::TransfersEnabled {
            transfers_timestamp: U64(1602614338293769340)
        };
        Ok(Some(state))
    } else {
        Ok(None)
    }
}
