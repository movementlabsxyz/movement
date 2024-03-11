//! Implements client for timestampvm APIs.

use std::{
    collections::HashMap,
    io::{self, Error, ErrorKind},
};

use avalanche_types::{ids, jsonrpc};
use serde::{Deserialize, Serialize};

/// Represents the RPC response for API `ping`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingResponse {
    pub jsonrpc: String,
    pub id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<crate::api::PingResponse>,

    /// Returns non-empty if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<APIError>,
}

/// Ping the VM.
/// # Errors
/// Errors on an http failure or a failed deserialization.
pub async fn ping(http_rpc: &str, url_path: &str) -> io::Result<PingResponse> {
    log::info!("ping {http_rpc} with {url_path}");

    let mut data = jsonrpc::RequestWithParamsArray::default();
    data.method = String::from("timestampvm.ping");

    let d = data.encode_json()?;
    let rb = http_manager::post_non_tls(http_rpc, url_path, &d).await?;

    serde_json::from_slice(&rb)
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed ping '{e}'")))
}

/// Represents the RPC response for API `last_accepted`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LastAcceptedResponse {
    pub jsonrpc: String,
    pub id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<crate::api::chain_handlers::LastAcceptedResponse>,

    /// Returns non-empty if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<APIError>,
}

/// Requests for the last accepted block Id.
/// # Errors
/// Errors on failed (de)serialization or an http failure.
pub async fn last_accepted(http_rpc: &str, url_path: &str) -> io::Result<LastAcceptedResponse> {
    log::info!("last_accepted {http_rpc} with {url_path}");

    let mut data = jsonrpc::RequestWithParamsArray::default();
    data.method = String::from("timestampvm.lastAccepted");

    let d = data.encode_json()?;
    let rb = http_manager::post_non_tls(http_rpc, url_path, &d).await?;

    serde_json::from_slice(&rb)
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed last_accepted '{e}'")))
}

/// Represents the RPC response for API `get_block`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBlockResponse {
    pub jsonrpc: String,
    pub id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<crate::api::chain_handlers::GetBlockResponse>,

    /// Returns non-empty if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<APIError>,
}

/// Fetches the block for the corresponding block Id (if any).
/// # Errors
/// Errors on failed (de)serialization or an http failure.
pub async fn get_block(
    http_rpc: &str,
    url_path: &str,
    id: &ids::Id,
) -> io::Result<GetBlockResponse> {
    log::info!("get_block {http_rpc} with {url_path}");

    let mut data = jsonrpc::RequestWithParamsHashMapArray::default();
    data.method = String::from("timestampvm.getBlock");

    let mut m = HashMap::new();
    m.insert("id".to_string(), id.to_string());

    let params = vec![m];
    data.params = Some(params);

    let d = data.encode_json()?;
    let rb = http_manager::post_non_tls(http_rpc, url_path, &d).await?;

    serde_json::from_slice(&rb)
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed get_block '{e}'")))
}

/// Represents the RPC response for API `propose_block`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposeBlockResponse {
    pub jsonrpc: String,
    pub id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<crate::api::chain_handlers::ProposeBlockResponse>,

    /// Returns non-empty if any.
    /// e.g., "error":{"code":-32603,"message":"data 1048586-byte exceeds the limit 1048576-byte"}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<APIError>,
}

/// Proposes arbitrary data.
/// # Errors
/// Errors on failed (de)serialization or an http failure.
pub async fn propose_block(
    http_rpc: &str,
    url_path: &str,
    d: Vec<u8>,
) -> io::Result<ProposeBlockResponse> {
    log::info!("propose_block {http_rpc} with {url_path}");

    let mut data = jsonrpc::RequestWithParamsHashMapArray::default();
    data.method = String::from("timestampvm.proposeBlock");

    let mut m = HashMap::new();
    m.insert(
        "data".to_string(),
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &d),
    );

    let params = vec![m];
    data.params = Some(params);

    let d = data.encode_json()?;
    let rb = http_manager::post_non_tls(http_rpc, url_path, &d).await?;

    serde_json::from_slice(&rb)
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed propose_block '{e}'")))
}

/// Represents the error (if any) for APIs.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APIError {
    pub code: i32,
    pub message: String,
}
