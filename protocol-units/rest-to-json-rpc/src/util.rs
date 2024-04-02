#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: serde_json::Value,
}


pub trait ToJsonRpc<T> {

    fn to_json_rpc(&self, request : T) -> Result<, anyhow::Error>;

}

pub mod warp {

    pub struct Warp;

    impl 

}