//! Implements static handlers specific to this VM.
//! To be served via `[HOST]/ext/vm/[VM ID]/static`.

use std::io;

use avalanche_types::{proto::http::Element, subnet::rpc::http::handle::Handle};
use bytes::Bytes;
use jsonrpc_core::{BoxFuture, IoHandler, Result};
use jsonrpc_derive::rpc;

use super::de_request;

/// Defines static handler RPCs for this VM.
#[rpc]
pub trait Rpc {
    #[rpc(name = "ping", alias("timestampvm.ping"))]
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>>;
}

/// Implements API services for the static handlers.
#[derive(Default)]
pub struct StaticService {}

impl StaticService {
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Rpc for StaticService {
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>> {
        log::debug!("ping called");
        Box::pin(async move { Ok(crate::api::PingResponse { success: true }) })
    }
}
#[derive(Clone)]
pub struct StaticHandler {
    pub handler: IoHandler,
}

impl StaticHandler {
    #[must_use]
    pub fn new(service: StaticService) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(Rpc::to_delegate(service));
        Self { handler }
    }
}

#[tonic::async_trait]
impl Handle for StaticHandler {
    async fn request(
        &self,
        req: &Bytes,
        _headers: &[Element],
    ) -> std::io::Result<(Bytes, Vec<Element>)> {
        match self.handler.handle_request(&de_request(req)?).await {
            Some(resp) => Ok((Bytes::from(resp), Vec::new())),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to handle request",
            )),
        }
    }
}
