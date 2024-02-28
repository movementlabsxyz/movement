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

/// Represents the error (if any) for APIs.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APIError {
    pub code: i32,
    pub message: String,
}
