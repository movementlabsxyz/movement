use super::client::{Config, MovementClient};
use super::utils::MovementAddress;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::types::Amount;
use crate::types::BridgeAddress;
use crate::types::BridgeTransferDetails;
use crate::types::BridgeTransferId;
use crate::types::HashLock;
use crate::types::HashLockPreImage;
use crate::types::LockDetails;
use anyhow::Result;
use aptos_sdk::rest_client::Response;
use aptos_types::contract_event::EventWithVersion;
use futures::channel::mpsc::{self};
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use serde::Deserialize;
use std::{pin::Pin, task::Poll};

pub struct MovementMonitoring {
	listener: mpsc::UnboundedReceiver<BridgeContractResult<BridgeContractEvent<MovementAddress>>>,
	client: MovementClient,
}

impl BridgeContractMonitoring for MovementMonitoring {
	type Address = MovementAddress;
}

impl MovementMonitoring {
	pub async fn build(config: Config) -> Result<Self, anyhow::Error> {
		let mvt_client = MovementClient::new(&config).await?;
		// Spawn a task to forward events to the listener channel
		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<MovementAddress>>,
		>();
		tokio::spawn({
			async move {
				let mvt_client = MovementClient::new(&config).await.unwrap();
				loop {
					let init_event_list = match pool_initiator_contract(&mvt_client).await {
						Ok(evs) => evs.into_iter().map(|ev| Ok(ev)).collect(),
						Err(err) => vec![Err(err)],
					};
					let counterpart_event_list = match pool_initiator_contract(&mvt_client).await {
						Ok(evs) => evs.into_iter().map(|ev| Ok(ev)).collect(),
						Err(err) => vec![Err(err)],
					};
					for event in
						init_event_list.into_iter().chain(counterpart_event_list.into_iter())
					{
						if sender.send(event).await.is_err() {
							tracing::error!("Failed to send event to listener channel");
							break;
						}
					}
				}
			}
		});

		Ok(MovementMonitoring { listener, client: mvt_client })
	}
}

impl Stream for MovementMonitoring {
	type Item = BridgeContractResult<BridgeContractEvent<MovementAddress>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.listener.poll_next_unpin(cx)
	}
}

enum InitiatorEventKind {
	Initiated,
	Completed,
	Refunded,
}

enum CounterpartyEventKind {
	Locked,
	Completed,
	Cancelled,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
struct CounterpartyCompletedDetails {
	pub bridge_transfer_id: BridgeTransferId,
	pub initiator_address: BridgeAddress<Vec<u8>>,
	pub recipient_address: BridgeAddress<MovementAddress>,
	pub hash_lock: HashLock,
	pub secret: HashLockPreImage,
	pub amount: Amount,
}

async fn pool_initiator_contract(
	client: &MovementClient,
) -> BridgeContractResult<Vec<BridgeContractEvent<MovementAddress>>> {
	let rest_client = client.rest_client();
	let struct_tag = format!(
		"0x{}::atomic_bridge_initiator::BridgeInitiatorEvents",
		client.native_address.to_standard_string(),
	);

	// Get initiated events
	let initiated_response = rest_client
		.get_account_events_bcs(
			client.native_address,
			struct_tag.as_str(),
			"bridge_transfer_initiated_events",
			Some(1),
			None,
		)
		.await
		.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	// Get completed events
	let completed_response = rest_client
		.get_account_events_bcs(
			client.native_address,
			struct_tag.as_str(),
			"bridge_transfer_completed_events",
			Some(1),
			None,
		)
		.await
		.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	// Get refunded events
	let refunded_response = rest_client
		.get_account_events_bcs(
			client.native_address,
			struct_tag.as_str(),
			"bridge_transfer_refunded_events",
			Some(1),
			None,
		)
		.await
		.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	// Process responses and yield events
	let initiated_events =
		process_initiator_response(initiated_response, InitiatorEventKind::Initiated)
			.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	let completed_events =
		process_initiator_response(completed_response, InitiatorEventKind::Completed)
			.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	let refunded_events =
		process_initiator_response(refunded_response, InitiatorEventKind::Refunded)
			.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	let total_events = initiated_events
		.into_iter()
		.chain(completed_events.into_iter())
		.chain(refunded_events.into_iter())
		.collect::<Vec<_>>();
	Ok(total_events)
}

async fn pool_counterpart_contract(
	client: &MovementClient,
) -> BridgeContractResult<Vec<BridgeContractEvent<MovementAddress>>> {
	let rest_client = client.rest_client();

	let struct_tag = format!(
		"0x{}::atomic_bridge_counterpary::BridgeCounterpartyEvents",
		client.native_address.to_standard_string()
	);

	// Get locked events
	let locked_response = rest_client
		.get_account_events_bcs(
			client.native_address,
			struct_tag.as_str(),
			"bridge_transfer_assets_locked",
			Some(1),
			None,
		)
		.await
		.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	// Get completed events
	let completed_response = rest_client
		.get_account_events_bcs(
			client.native_address,
			struct_tag.as_str(),
			"bridge_transfer_completed",
			Some(1),
			None,
		)
		.await
		.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	// Get cancelled events
	let cancelled_response = rest_client
		.get_account_events_bcs(
			client.native_address,
			struct_tag.as_str(),
			"bridge_transfer_cancelled",
			Some(1),
			None,
		)
		.await
		.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	// Process responses and return results
	let locked_events =
		process_counterparty_response(locked_response, CounterpartyEventKind::Locked)
			.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	let completed_events =
		process_counterparty_response(completed_response, CounterpartyEventKind::Completed)
			.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	let cancelled_events =
		process_counterparty_response(cancelled_response, CounterpartyEventKind::Cancelled)
			.map_err(|e| BridgeContractError::OnChainError(e.to_string()))?;

	let total_events = locked_events
		.into_iter()
		.chain(completed_events.into_iter())
		.chain(cancelled_events.into_iter())
		.collect::<Vec<_>>();
	Ok(total_events)
}

fn process_initiator_response(
	res: Response<Vec<EventWithVersion>>,
	kind: InitiatorEventKind,
) -> Result<Vec<BridgeContractEvent<MovementAddress>>, bcs::Error> {
	//TODO error management should be done differently because if one event fail all other non processed one are lost.
	res.into_inner()
		.into_iter()
		.map(|e| {
			let data = e.event.event_data();
			match kind {
				InitiatorEventKind::Initiated => {
					let transfer_details =
						bcs::from_bytes::<BridgeTransferDetails<MovementAddress>>(data)?;
					Ok(BridgeContractEvent::Initiated(transfer_details))
				}
				InitiatorEventKind::Completed => {
					let completed_details = bcs::from_bytes::<CounterpartyCompletedDetails>(data)?;
					Ok(BridgeContractEvent::InitialtorCompleted(
						completed_details.bridge_transfer_id,
					))
				}
				InitiatorEventKind::Refunded => {
					let completed_details = bcs::from_bytes::<CounterpartyCompletedDetails>(data)?;
					Ok(BridgeContractEvent::Refunded(completed_details.bridge_transfer_id))
				}
			}
		})
		.collect()
}

fn process_counterparty_response(
	res: Response<Vec<EventWithVersion>>,
	kind: CounterpartyEventKind,
) -> Result<Vec<BridgeContractEvent<MovementAddress>>, bcs::Error> {
	res.into_inner()
		.into_iter()
		.map(|e| {
			let data = e.event.event_data();
			match kind {
				CounterpartyEventKind::Locked => {
					let locked_details = bcs::from_bytes::<LockDetails<MovementAddress>>(data)?;
					Ok(BridgeContractEvent::Locked(locked_details))
				}
				CounterpartyEventKind::Completed => {
					let completed_details = bcs::from_bytes::<CounterpartyCompletedDetails>(data)?;
					Ok(BridgeContractEvent::CounterPartCompleted(
						completed_details.bridge_transfer_id,
					))
				}
				CounterpartyEventKind::Cancelled => {
					let completed_details = bcs::from_bytes::<CounterpartyCompletedDetails>(data)?;
					Ok(BridgeContractEvent::CounterPartCompleted(
						completed_details.bridge_transfer_id,
					))
				}
			}
		})
		.collect()
}
