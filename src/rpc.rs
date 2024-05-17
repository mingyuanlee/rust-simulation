// src/rpc.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: serde_json::Value,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcError {
    pub jsonrpc: String,
    pub error: ErrorObject,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
