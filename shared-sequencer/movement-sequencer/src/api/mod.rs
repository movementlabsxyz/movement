//! Implementation of timestampvm APIs, to be registered via
//! `create_static_handlers` and `create_handlers` in the [`vm`](crate::vm) crate.

pub mod chain_handlers;
pub mod static_handlers;

use std::io;

use bytes::Bytes;
use jsonrpc_core::MethodCall;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PingResponse {
    pub success: bool,
}

/// Deserializes JSON-RPC method call.
/// # Errors
/// Fails if the request is not a valid JSON-RPC method call.
pub fn de_request(req: &Bytes) -> io::Result<String> {
    let method_call: MethodCall = serde_json::from_slice(req).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to deserialize request: {e}"),
        )
    })?;
    serde_json::to_string(&method_call).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to serialize request: {e}"),
        )
    })
}
