//! Implements chain/VM specific handlers.
//! To be served via `[HOST]/ext/bc/[CHAIN ID]/rpc`.

use crate::{block::Block, vm::Vm};
use avalanche_types::{ids, proto::http::Element, subnet::rpc::http::handle::Handle};
use bytes::Bytes;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, IoHandler, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, io, marker::PhantomData, str::FromStr};

use super::de_request;

/// Defines RPCs specific to the chain.
#[rpc]
pub trait Rpc {
    /// Pings the VM.
    #[rpc(name = "ping", alias("timestampvm.ping"))]
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>>;

    /// Proposes the arbitrary data.
    #[rpc(name = "proposeBlock", alias("timestampvm.proposeBlock"))]
    fn propose_block(&self, args: ProposeBlockArgs) -> BoxFuture<Result<ProposeBlockResponse>>;

    /// Fetches the last accepted block.
    #[rpc(name = "lastAccepted", alias("timestampvm.lastAccepted"))]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    /// Fetches the block.
    #[rpc(name = "getBlock", alias("timestampvm.getBlock"))]
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProposeBlockArgs {
    #[serde(with = "avalanche_types::codec::serde::base64_bytes")]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProposeBlockResponse {
    /// TODO: returns Id for later query, using hash + time?
    pub success: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LastAcceptedResponse {
    pub id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockArgs {
    /// TODO: use "ids::Id"
    /// if we use "ids::Id", it fails with:
    /// "Invalid params: invalid type: string \"g25v3qDyAaHfR7kBev8tLUHouSgN5BJuZjy1BYS1oiHd2vres\", expected a borrowed string."
    pub id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockResponse {
    pub block: Block,
}

/// Implements API services for the chain-specific handlers.
#[derive(Clone)]
pub struct ChainService<A> {
    pub vm: Vm<A>,
}

impl<A> ChainService<A> {
    pub fn new(vm: Vm<A>) -> Self {
        Self { vm }
    }
}

impl<A> Rpc for ChainService<A>
where
    A: Send + Sync + Clone + 'static,
{
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>> {
        log::debug!("ping called");
        Box::pin(async move { Ok(crate::api::PingResponse { success: true }) })
    }

    fn propose_block(&self, args: ProposeBlockArgs) -> BoxFuture<Result<ProposeBlockResponse>> {
        log::debug!("propose_block called");
        let vm = self.vm.clone();

        Box::pin(async move {
            vm.propose_block(args.data)
                .await
                .map_err(create_jsonrpc_error)?;
            Ok(ProposeBlockResponse { success: true })
        })
    }

    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>> {
        log::debug!("last accepted method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let vm_state = vm.state.read().await;
            if let Some(state) = &vm_state.state {
                let last_accepted = state
                    .get_last_accepted_block_id()
                    .await
                    .map_err(create_jsonrpc_error)?;

                return Ok(LastAcceptedResponse { id: last_accepted });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }

    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>> {
        let blk_id = ids::Id::from_str(&args.id).unwrap();
        log::info!("get_block called for {}", blk_id);

        let vm = self.vm.clone();

        Box::pin(async move {
            let vm_state = vm.state.read().await;
            if let Some(state) = &vm_state.state {
                let block = state
                    .get_block(&blk_id)
                    .await
                    .map_err(create_jsonrpc_error)?;

                return Ok(GetBlockResponse { block });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChainHandler<T> {
    pub handler: IoHandler,
    _marker: PhantomData<T>,
}

impl<T: Rpc> ChainHandler<T> {
    pub fn new(service: T) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(Rpc::to_delegate(service));
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}

#[tonic::async_trait]
impl<T> Handle for ChainHandler<T>
where
    T: Rpc + Send + Sync + Clone + 'static,
{
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

fn create_jsonrpc_error<E: Borrow<std::io::Error>>(e: E) -> Error {
    let e = e.borrow();
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{e}");
    error
}
