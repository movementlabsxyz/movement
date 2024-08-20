use crate::MovementClient;
use crate::{
	event_types::{CounterpartyEventKind, MovementChainEvent},
	types::MovementHash,
	utils::MovementAddress,
};
use anyhow::Error;
use anyhow::Result;
use aptos_sdk::rest_client::Response;
use aptos_types::contract_event::EventWithVersion;
use async_stream::try_stream;
use bridge_shared::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
};
use bridge_shared::types::{CounterpartyCompletedDetails, LockDetails};
use futures::{Stream, TryFuture};
use std::{pin::Pin, task::Poll};
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

#[allow(unused)]
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
		let client = if let Some(client) = self.client.as_ref() {
			client
		} else {
			return Poll::Ready(None);
		};

		let rest_client = client.rest_client();
		let stream = try_stream! {
			loop {
				let struct_tag = format!(
					"0x{}::atomic_bridge_counterpary::BridgeCounterpartyEvents",
					client.counterparty_address.to_hex_literal()
				);

				// Get locked events
				let locked_response = rest_client
					.get_account_events_bcs(
						client.counterparty_address,
						struct_tag.as_str(),
						"bridge_transfer_assets_locked",
						Some(1),
						None,
					)
					.await
					.map_err(|e| Error::msg(e.to_string()))?;

				// Get completed events
				let completed_response = rest_client
					.get_account_events_bcs(
						client.counterparty_address,
						struct_tag.as_str(),
						"bridge_transfer_completed",
						Some(1),
						None,
					)
					.await
					.map_err(|e| Error::msg(e.to_string()))?;

				// Get cancelled events
				let cancelled_response = rest_client
					.get_account_events_bcs(
						client.counterparty_address,
						struct_tag.as_str(),
						"bridge_transfer_cancelled",
						Some(1),
						None,
					)
					.await
					.map_err(|e| Error::msg(e.to_string()))?;

				// Process responses and return results
				let locked_events = process_response(locked_response, CounterpartyEventKind::Locked)
					.map_err(|e| Error::msg(e.to_string()))?;

				let completed_events = process_response(completed_response, CounterpartyEventKind::Completed)
					.map_err(|e| Error::msg(e.to_string()))?;

				let cancelled_events = process_response(cancelled_response, CounterpartyEventKind::Cancelled)
					.map_err(|e| Error::msg(e.to_string()))?;

				let total_events = locked_events
					.into_iter()
					.chain(completed_events.into_iter())
					.chain(cancelled_events.into_iter())
					.collect::<Vec<_>>();

				for event in total_events {
					yield event;
				}
			}
		};

		// We need to coerce and declare the reutned type of `try_stream!`
		#[allow(clippy::type_complexity)]
		let mut stream: Pin<
			Box<
				dyn Stream<
						Item = Result<
							BridgeContractCounterpartyEvent<
								<Self as BridgeContractCounterpartyMonitoring>::Address,
								<Self as BridgeContractCounterpartyMonitoring>::Hash,
							>,
							Error,
						>,
					> + Send,
			>,
		> = Box::pin(stream);

		// Poll the stream to get the next event
		match Pin::new(&mut stream).poll_next(cx) {
			Poll::Ready(Some(Ok(event))) => Poll::Ready(Some(event)),
			Poll::Ready(Some(Err(_))) => Poll::Ready(None),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

fn process_response(
	res: Response<Vec<EventWithVersion>>,
	kind: CounterpartyEventKind,
) -> Result<Vec<BridgeContractCounterpartyEvent<MovementAddress, MovementHash>>, bcs::Error> {
	res.into_inner()
		.into_iter()
		.map(|e| {
			let data = e.event.event_data();
			match kind {
				CounterpartyEventKind::Locked => {
					let locked_details =
						bcs::from_bytes::<LockDetails<MovementAddress, [u8; 32]>>(data)?;
					Ok(BridgeContractCounterpartyEvent::Locked(locked_details))
				}
				CounterpartyEventKind::Completed => {
					let completed_details = bcs::from_bytes::<
						CounterpartyCompletedDetails<MovementAddress, [u8; 32]>,
					>(data)?;
					Ok(BridgeContractCounterpartyEvent::Completed(completed_details))
				}
				CounterpartyEventKind::Cancelled => {
					let completed_details = bcs::from_bytes::<
						CounterpartyCompletedDetails<MovementAddress, [u8; 32]>,
					>(data)?;
					Ok(BridgeContractCounterpartyEvent::Completed(completed_details))
				}
			}
		})
		.collect()
}
