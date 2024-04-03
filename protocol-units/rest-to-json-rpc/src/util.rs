use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}


pub trait ToJsonRpc<T> {

    // ? This currently does not need to be part of the trait. It's just this way for organization.
    /// Converts a request to a method name
    fn request_to_method(&self, request : &T) -> Result<String, anyhow::Error>;

    /// Converts a request to a JsonRpcRequest
    fn to_json_rpc(&self, request : T) -> Result<JsonRpcRequest, anyhow::Error>;

}

pub trait Middleware<T> {
    fn apply(&self, request : T) -> Result<T, anyhow::Error>;
}