use std::{pin::Pin, task::Poll};

use crate::{event_types::MovementChainEvent, types::MovementHash, utils::MovementAddress};
use bridge_shared::{
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring},
	counterparty_contract::SmartContractCounterpartyEvent,
};
use futures::{FutureExt, Stream, StreamExt};
use tokio::sync::mpsc::UnboundedReceiver;

#[allow(unused)]
pub struct MovementCounterpartyMonitoring<A, H> {
	listener: UnboundedReceiver<MovementChainEvent<A, H>>,
	//ws: Roo<PubSubFrontend>,
}

impl BridgeContractCounterpartyMonitoring
	for MovementCounterpartyMonitoring<MovementAddress, MovementHash>
{
	type Address = MovementAddress;
	type Hash = MovementHash;
}

impl MovementCounterpartyMonitoring<MovementAddress, MovementHash> {
	async fn run(
		_rpc_url: &str,
		_listener: UnboundedReceiver<MovementChainEvent<MovementAddress, MovementHash>>,
	) -> Result<Self, anyhow::Error> {
		todo!()
	}
}

impl Stream for MovementCounterpartyMonitoring<MovementAddress, MovementHash> {
	type Item = BridgeContractCounterpartyEvent<
		<Self as BridgeContractCounterpartyMonitoring>::Address,
		<Self as BridgeContractCounterpartyMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		todo!()
	}
}
