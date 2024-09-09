use crate::client::MovementClient;
use crate::event_types::InitiatorEventKind;
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
	BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
};
use bridge_shared::types::{BridgeTransferDetails, CounterpartyCompletedDetails, LockDetails};
use futures::Stream;
use std::{pin::Pin, task::Poll};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct MovementInitiatorMonitoring<A, H> {
	listener: UnboundedReceiver<MovementChainEvent<A, H>>,
	client: Option<MovementClient>,
}

impl BridgeContractInitiatorMonitoring
	for MovementInitiatorMonitoring<MovementAddress, MovementHash>
{
	type Address = MovementAddress;
	type Hash = MovementHash;
}

impl MovementInitiatorMonitoring<MovementAddress, MovementHash> {
	pub async fn build(
		rest_url: &str,
		listener: UnboundedReceiver<MovementChainEvent<MovementAddress, MovementHash>>,
	) -> Result<Self, anyhow::Error> {
		todo!()
	}
}

impl Stream for MovementInitiatorMonitoring<MovementAddress, MovementHash> {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
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
					"0x{}::atomic_bridge_initiator::BridgeInitiatorEvents",
					client.initiator_address.to_hex_literal()
				);

				// Get initiated events
				let initiated_response = rest_client
					.get_account_events_bcs(
						client.initiator_address,
						struct_tag.as_str(),
						"bridge_transfer_initiated_events",
						Some(1),
						None,
					)
					.await
					.map_err(|e| Error::msg(e.to_string()))?;

				// Get completed events
				let completed_response = rest_client
					.get_account_events_bcs(
						client.initiator_address,
						struct_tag.as_str(),
						"bridge_transfer_completed_events",
						Some(1),
						None,
					)
					.await
					.map_err(|e| Error::msg(e.to_string()))?;

				// Get refunded events
				let refunded_response = rest_client
					.get_account_events_bcs(
						client.initiator_address,
						struct_tag.as_str(),
						"bridge_transfer_refunded_events",
						Some(1),
						None,
					)
					.await
					.map_err(|e| Error::msg(e.to_string()))?;

				// Process responses and yield events
				let initiated_events = process_initiator_response(initiated_response, InitiatorEventKind::Initiated)
					.map_err(|e| Error::msg(e.to_string()))?;

				let completed_events = process_initiator_response(completed_response, InitiatorEventKind::Completed)
					.map_err(|e| Error::msg(e.to_string()))?;

				let refunded_events = process_initiator_response(refunded_response, InitiatorEventKind::Refunded)
					.map_err(|e| Error::msg(e.to_string()))?;

				let total_events = initiated_events
					.into_iter()
					.chain(completed_events.into_iter())
					.chain(refunded_events.into_iter())
					.collect::<Vec<_>>();

				for event in total_events {
					yield event;
				}
			}
		};

		// We need to coerce and declare the returned type of `try_stream!`
		#[allow(clippy::type_complexity)]
		let mut stream: Pin<
			Box<
				dyn Stream<
						Item = Result<
							BridgeContractInitiatorEvent<
								<Self as BridgeContractInitiatorMonitoring>::Address,
								<Self as BridgeContractInitiatorMonitoring>::Hash,
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
	pub async fn build(
		rest_url: &str,
		listener: UnboundedReceiver<MovementChainEvent<MovementAddress, MovementHash>>,
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
				let locked_events = process_counterparty_response(locked_response, CounterpartyEventKind::Locked)
					.map_err(|e| Error::msg(e.to_string()))?;

				let completed_events = process_counterparty_response(completed_response, CounterpartyEventKind::Completed)
					.map_err(|e| Error::msg(e.to_string()))?;

				let cancelled_events = process_counterparty_response(cancelled_response, CounterpartyEventKind::Cancelled)
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

fn process_initiator_response(
	res: Response<Vec<EventWithVersion>>,
	kind: InitiatorEventKind,
) -> Result<Vec<BridgeContractInitiatorEvent<MovementAddress, MovementHash>>, bcs::Error> {
	res.into_inner()
		.into_iter()
		.map(|e| {
			let data = e.event.event_data();
			match kind {
				InitiatorEventKind::Initiated => {
					let transfer_details = bcs::from_bytes::<
						BridgeTransferDetails<MovementAddress, MovementHash>,
					>(data)?;
					Ok(BridgeContractInitiatorEvent::Initiated(transfer_details))
				}
				InitiatorEventKind::Completed => {
					let completed_details = bcs::from_bytes::<
						CounterpartyCompletedDetails<MovementAddress, [u8; 32]>,
					>(data)?;
					Ok(BridgeContractInitiatorEvent::Completed(
						completed_details.bridge_transfer_id,
					))
				}
				InitiatorEventKind::Refunded => {
					let completed_details = bcs::from_bytes::<
						CounterpartyCompletedDetails<MovementAddress, [u8; 32]>,
					>(data)?;
					Ok(BridgeContractInitiatorEvent::Refunded(completed_details.bridge_transfer_id))
				}
			}
		})
		.collect()
}

fn process_counterparty_response(
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
