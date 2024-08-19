use std::{pin::Pin, task::Poll};

use crate::MovementClient;
use crate::{event_types::MovementChainEvent, types::MovementHash, utils::MovementAddress};
use async_stream::try_stream;
use bridge_shared::{
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring},
	counterparty_contract::SmartContractCounterpartyEvent,
};
use futures::{FutureExt, Stream, StreamExt};
use tokio::sync::mpsc::UnboundedReceiver;

#[allow(unused)]
pub struct MovementCounterpartyMonitoring<A, H> {
	listener: UnboundedReceiver<MovementChainEvent<A, H>>,
	client: Option<MovementClient>,
}

impl BridgeContractCounterpartyMonitoring
	for MovementCounterpartyMonitoring<MovementAddress, MovementHash>
{
	type Address = MovementAddress;
	type Hash = MovementHash;
}

impl MovementCounterpartyMonitoring<MovementAddress, MovementHash> {
	async fn run(
		_rest_url: &str,
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
		let client = self.client.as_ref().unwrap(); // would be nice if poll_next could return Result
		let rest_client = client.rest_client();
		let this = self.get_mut();
		let stream = try_stream! {
			loop {
				let struct_tag = format!("0x{}::atomic_bridge_counterpary::BridgeTransferAssetsLockedEvent", client.counterparty_address.to_hex_literal());
				rest_client.get_account_events_bcs(client.counterparty_address, struct_tag.as_str(), field_name, start, limit);
				let response = reqwest::get("https://httpbin.org/ip").await.map_err(|e| {
					tracing::error!("Failed to get response: {:?}", e);
					Error::msg("Failed to get response")
				})?;

				// Create an event from the response (replace with actual logic)
				let event = BridgeContractCounterpartyEvent {
					// Populate event fields based on the response
				};

				// Yield the event
				yield Ok(event);
			}
		};

		let mut stream = Box::pin(stream);
		Pin::new(&mut stream).poll_next(cx)
	}
}
