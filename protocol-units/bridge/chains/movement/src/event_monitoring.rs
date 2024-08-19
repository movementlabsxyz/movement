use std::{pin::Pin, task::Poll};

use crate::MovementClient;
use crate::{event_types::MovementChainEvent, types::MovementHash, utils::MovementAddress};
use anyhow::Result;
use aptos_sdk::rest_client::Response;
use aptos_types::contract_event::{ContractEvent, ContractEventV1, EventWithVersion};
use async_stream::try_stream;
use bridge_shared::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
};
use bridge_shared::types::CounterpartyCompletedDetails;
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
				let struct_tag = format!(
						"0x{}::atomic_bridge_counterpary::BridgeCounterpartyEvents",
						client.counterparty_address.to_hex_literal()
				);
				let response = rest_client
								.get_account_events_bcs(
										client.counterparty_address,
										struct_tag.as_str(),
										"bridge_transfer_assets_locked",
										Some(1),
										None
								).await?;
						let events = process_response(response);
				let bridge_transfer_details = bcs::from_bytes::<CounterpartyCompletedDetails<MovementAddress, MovementHash>>(
						&response.event_data
				);

				// Yield the event
				yield Ok(events);
			}
		};

		let mut stream = Box::pin(stream);
		Pin::new(&mut stream).poll_next(cx)
	}
}

fn process_response(
	res: Response<Vec<EventWithVersion>>,
) -> Result<Vec<CounterpartyCompletedDetails<MovementAddress, MovementHash>>, bcs::Error> {
	res.into_inner()
		.into_iter()
		.map(|e| {
			let event_data = e.event.event_data(); // Use the method from the trait
			bcs::from_bytes::<CounterpartyCompletedDetails<MovementAddress, MovementHash>>(
				event_data,
			)
		})
		.collect() // Collect the results, handling potential errors
}
