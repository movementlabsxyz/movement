use super::types::EthAddress;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::chains::ethereum::types::AtomicBridgeCounterpartyMOVE;
use crate::chains::ethereum::types::AtomicBridgeInitiatorMOVE;
use crate::types::HashLockPreImage;
use crate::types::LockDetails;
use crate::types::{BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy_network::EthereumWallet;
use bridge_config::common::eth::EthConfig;
use futures::SinkExt;
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use std::{pin::Pin, task::Poll};

pub struct EthMonitoring {
	listener: UnboundedReceiver<BridgeContractResult<BridgeContractEvent<EthAddress>>>,
}

impl BridgeContractMonitoring for EthMonitoring {
	type Address = EthAddress;
}

impl EthMonitoring {
	pub async fn build(config: &EthConfig) -> Result<Self, anyhow::Error> {
		let client_config: crate::chains::ethereum::client::Config = config.try_into()?;
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(client_config.signer_private_key.clone()))
			.on_builtin(client_config.rpc_url.as_str())
			.await?;

		tracing::info!(
			"Start Eth monitoring with initiator:{} counterpart:{}",
			config.eth_initiator_contract,
			config.eth_counterparty_contract
		);

		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<EthAddress>>,
		>();

		tokio::spawn({
			let config = config.clone();
			async move {
				let initiator_contract = AtomicBridgeInitiatorMOVE::new(
					config.eth_initiator_contract.parse().unwrap(),
					rpc_provider.clone(),
				);
				let counterpart_contract = AtomicBridgeCounterpartyMOVE::new(
					config.eth_counterparty_contract.parse().unwrap(),
					rpc_provider.clone(),
				);
				//We start at the current block.
				//TODO save the start between restart.
				let mut last_processed_block = 0;
				loop {
					let block_number = rpc_provider.get_block_number().await.unwrap();
					if last_processed_block < block_number {
						last_processed_block = block_number;
						let initiator_initiate_event_filter = initiator_contract
							.BridgeTransferInitiated_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						// event BridgeTransferCompleted(bytes32 indexed _bridgeTransferId, bytes32 pre_image);
						let initiator_trcompleted_event_filter = initiator_contract
							.BridgeTransferCompleted_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						// event BridgeTransferRefunded(bytes32 indexed _bridgeTransferId);
						let initiator_trrefund_event_filter = initiator_contract
							.BridgeTransferRefunded_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						let counterpart_trlocked_event_filter = counterpart_contract
							.BridgeTransferLocked_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						let counterpart_trcompleted_event_filter = counterpart_contract
							.BridgeTransferCompleted_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						//event BridgeTransferAborted(bytes32 indexed bridgeTransferId);
						let counterpart_trcaborted_event_filter = counterpart_contract
							.BridgeTransferAborted_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));

						//Initiator event stream
						match initiator_initiate_event_filter.query().await {
							Ok(events) => {
								for (initiated, _log) in events {
									let event = {
										// BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
										let details: BridgeTransferDetails<EthAddress> =
											BridgeTransferDetails {
												bridge_transfer_id: BridgeTransferId(
													*initiated._bridgeTransferId,
												),
												initiator_address: BridgeAddress(EthAddress(
													Address::from(initiated._originator),
												)),
												recipient_address: BridgeAddress(
													initiated._recipient.to_vec(),
												),
												hash_lock: HashLock(*initiated._hashLock),
												time_lock: initiated._timeLock.into(),
												amount: initiated.amount.into(),
												state: 0,
											};
										BridgeContractEvent::Initiated(details)
									};
									if sender.send(Ok(event)).await.is_err() {
										tracing::error!("Failed to send event to listener channel");
										break;
									}
								}
							}
							Err(err) => {
								if sender
									.send(Err(BridgeContractError::OnChainError(err.to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
								}
							}
						}
						match initiator_trcompleted_event_filter.query().await {
							Ok(events) => {
								for (completed, _log) in events {
									if sender
										.send(Ok(BridgeContractEvent::InitialtorCompleted(
											BridgeTransferId(*completed._bridgeTransferId),
										)))
										.await
										.is_err()
									{
										tracing::error!("Failed to send event to listener channel");
										break;
									}
								}
							}
							Err(err) => {
								if sender
									.send(Err(BridgeContractError::OnChainError(err.to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
								}
							}
						}
						match initiator_trrefund_event_filter.query().await {
							Ok(events) => {
								for (refund, _log) in events {
									if sender
										.send(Ok(BridgeContractEvent::Refunded(BridgeTransferId(
											*refund._bridgeTransferId,
										))))
										.await
										.is_err()
									{
										tracing::error!("Failed to send event to listener channel");
										break;
									}
								}
							}
							Err(err) => {
								if sender
									.send(Err(BridgeContractError::OnChainError(err.to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
								}
							}
						}
						match counterpart_trlocked_event_filter.query().await {
							Ok(events) => {
								for (trlocked, _log) in events {
									let event = {
										// BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
										let details: LockDetails<EthAddress> = LockDetails {
											bridge_transfer_id: BridgeTransferId(
												*trlocked.bridgeTransferId,
											),
											initiator: BridgeAddress([0, 32].into()), // TODO add the originator fields. trlocked.originator.to_vec()
											recipient: BridgeAddress(EthAddress(Address::from(
												trlocked.recipient,
											))),
											amount: trlocked.amount.into(),
											hash_lock: HashLock(*trlocked.hashLock),
											time_lock: trlocked.timeLock.into(),
										};
										BridgeContractEvent::Locked(details)
									};
									if sender.send(Ok(event)).await.is_err() {
										tracing::error!("Failed to send event to listener channel");
										break;
									}
								}
							}
							Err(err) => {
								if sender
									.send(Err(BridgeContractError::OnChainError(err.to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
								}
							}
						}
						match counterpart_trcompleted_event_filter.query().await {
							Ok(events) => {
								for (completed, _log) in events {
									if sender
										.send(Ok(BridgeContractEvent::CounterPartCompleted(
											BridgeTransferId(*completed.bridgeTransferId),
											HashLockPreImage(*completed.pre_image),
										)))
										.await
										.is_err()
									{
										tracing::error!("Failed to send event to listener channel");
										break;
									}
								}
							}
							Err(err) => {
								if sender
									.send(Err(BridgeContractError::OnChainError(err.to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
								}
							}
						}
						match counterpart_trcaborted_event_filter.query().await {
							Ok(events) => {
								for (aborted, _log) in events {
									if sender
										.send(Ok(BridgeContractEvent::Cancelled(BridgeTransferId(
											*aborted.bridgeTransferId,
										))))
										.await
										.is_err()
									{
										tracing::error!("Failed to send event to listener channel");
										break;
									}
								}
							}
							Err(err) => {
								if sender
									.send(Err(BridgeContractError::OnChainError(err.to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
								}
							}
						}
					}
					let _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
				}
			}
		});

		Ok(Self { listener })
	}
}

impl Stream for EthMonitoring {
	type Item = BridgeContractResult<BridgeContractEvent<EthAddress>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.listener.poll_next_unpin(cx)
	}
}

// fn decode_initiator_log_data(log: Log) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
// 	let topics = log.topics().to_owned();
// 	let log_data =
// 		LogData::new(topics.clone(), log.data().data.clone()).expect("Failed to create log data");

// 	// Build the event
// 	let event = topics
// 		.iter()
// 		.find_map(|topic| {
// 			match *topic {
// 				INITIATOR_INITIATED_SELECT => Some(Event {
// 					name: EventName::InitiatorInitiated.as_str().to_string(),
// 					inputs: EventName::InitiatorInitiated
// 						.params()
// 						.iter()
// 						.map(|p| EventParam {
// 							ty: p.to_string(),
// 							name: EventName::InitiatorCompleted.as_str().to_string(),
// 							indexed: true,
// 							components: EventName::InitiatorInitiated.params(),
// 							internal_type: None, // for now
// 						})
// 						.collect(),
// 					anonymous: false,
// 				}),
// 				INITIATOR_COMPLETED_SELECT => Some(Event {
// 					name: EventName::InitiatorCompleted.as_str().to_string(),
// 					inputs: EventName::InitiatorCompleted
// 						.params()
// 						.iter()
// 						.map(|p| EventParam {
// 							ty: p.to_string(),
// 							name: p.name.clone(),
// 							indexed: true,
// 							components: EventName::InitiatorCompleted.params(),
// 							internal_type: None, // for now
// 						})
// 						.collect(),
// 					anonymous: false,
// 				}),
// 				INITIATOR_REFUNDED_SELECT => Some(Event {
// 					name: EventName::InitiatorRefunded.as_str().to_string(),
// 					inputs: EventName::InitiatorRefunded
// 						.params()
// 						.iter()
// 						.map(|p| EventParam {
// 							ty: p.to_string(),
// 							name: p.name.clone(),
// 							indexed: true,
// 							components: EventName::InitiatorRefunded.params(),
// 							internal_type: None, // for now
// 						})
// 						.collect(),
// 					anonymous: false,
// 				}),
// 				_ => None,
// 			}
// 		})
// 		.ok_or_else(|| BridgeContractError::OnChainUnknownEvent)?;

// 	let decoded = event.decode_log(&log_data, true).expect("Failed to decode log");

// 	let coerce_bytes = |(bytes, _): (&[u8], usize)| {
// 		let mut array = [0u8; 32];
// 		array.copy_from_slice(bytes);
// 		array
// 	};

// 	if let Some(selector) = decoded.selector {
// 		match selector {
// 			INITIATOR_INITIATED_SELECT => {
// 				let bridge_transfer_id =
// 					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed("BridgeTransferId".to_string())
// 					})?;
// 				let initiator_address =
// 					decoded.indexed[1].as_address().map(EthAddress).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed("InitiatorAddress".to_string())
// 					})?;
// 				let recipient_address =
// 					decoded.indexed[2].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed("RecipientAddress".to_string())
// 					})?;
// 				let amount = decoded.indexed[3]
// 					.as_uint()
// 					.map(|(u, _)| u.into())
// 					.ok_or_else(|| BridgeContractError::ConversionFailed("Amount".to_string()))?;
// 				let hash_lock = decoded.indexed[4]
// 					.as_fixed_bytes()
// 					.map(coerce_bytes)
// 					.ok_or_else(|| BridgeContractError::ConversionFailed("HashLock".to_string()))?;
// 				let time_lock = decoded.indexed[5]
// 					.as_uint()
// 					.map(|(u, _)| u.into())
// 					.ok_or_else(|| BridgeContractError::ConversionFailed("TimeLock".to_string()))?;
// 				let state = decoded
// 					.indexed
// 					.get(6)
// 					.and_then(|val| val.as_uint())
// 					.and_then(|(u, _)| u.try_into().ok()) // Try converting to u8
// 					.ok_or_else(|| {
// 						BridgeContractError::ConversionFailed(
// 							"Failed to decode state as u8".to_string(),
// 						)
// 					})?;

// 				let details: BridgeTransferDetails<EthAddress> = BridgeTransferDetails {
// 					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
// 					initiator_address: BridgeAddress(initiator_address),
// 					recipient_address: BridgeAddress(recipient_address.to_vec()),
// 					hash_lock: HashLock(hash_lock),
// 					time_lock,
// 					amount,
// 					state,
// 				};

// 				Ok(BridgeContractEvent::Initiated(details))
// 			}
// 			INITIATOR_COMPLETED_SELECT => {
// 				let bridge_transfer_id =
// 					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed(
// 							"Failed to decode BridgeTransferId".to_string(),
// 						)
// 					})?;

// 				Ok(BridgeContractEvent::InitialtorCompleted(BridgeTransferId(bridge_transfer_id)))
// 			}
// 			INITIATOR_REFUNDED_SELECT => {
// 				let bridge_transfer_id =
// 					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed("BridgeTransferId".to_string())
// 					})?;

// 				Ok(BridgeContractEvent::Refunded(BridgeTransferId(bridge_transfer_id)))
// 			}
// 			_ => {
// 				tracing::error!("Unknown event selector: {:x}", selector);
// 				Err(BridgeContractError::ConversionFailed("event selector".to_string()))
// 			}
// 		}
// 	} else {
// 		tracing::error!("Failed to decode event selector");
// 		Err(BridgeContractError::ConversionFailed("event selector".to_string()))
// 	}
// }

// fn decode_counterparty_log_data(log: Log) -> BridgeContractResult<BridgeContractEvent<EthAddress>> {
// 	let topics = log.topics().to_owned();
// 	let log_data =
// 		LogData::new(topics.clone(), log.data().data.clone()).expect("Failed to create log data");

// 	// Build the event
// 	let event = topics
// 		.iter()
// 		.find_map(|topic| {
// 			match *topic {
// 				COUNTERPARTY_LOCKED_SELECT => Some(Event {
// 					name: EventName::CounterpartyLocked.as_str().to_string(),
// 					inputs: EventName::CounterpartyLocked
// 						.params()
// 						.iter()
// 						.map(|p| EventParam {
// 							ty: p.to_string(),
// 							name: p.name.clone(),
// 							indexed: true,
// 							components: EventName::CounterpartyLocked.params(),
// 							internal_type: None, // for now
// 						})
// 						.collect(),
// 					anonymous: false,
// 				}),
// 				COUNTERPARTY_COMPLETED_SELECT => Some(Event {
// 					name: EventName::CounterpartyCompleted.as_str().to_string(),
// 					inputs: EventName::CounterpartyCompleted
// 						.params()
// 						.iter()
// 						.map(|p| EventParam {
// 							ty: p.to_string(),
// 							name: p.name.clone(),
// 							indexed: true,
// 							components: EventName::CounterpartyCompleted.params(),
// 							internal_type: None, // for now
// 						})
// 						.collect(),
// 					anonymous: false,
// 				}),
// 				COUNTERPARTY_ABORTED_SELECT => Some(Event {
// 					name: EventName::CounterpartyAborted.as_str().to_string(),
// 					inputs: EventName::CounterpartyAborted
// 						.params()
// 						.iter()
// 						.map(|p| EventParam {
// 							ty: p.to_string(),
// 							name: p.name.clone(),
// 							indexed: true,
// 							components: EventName::CounterpartyAborted.params(),
// 							internal_type: None, // for now
// 						})
// 						.collect(),
// 					anonymous: false,
// 				}),
// 				_ => None,
// 			}
// 		})
// 		.ok_or_else(|| BridgeContractError::OnChainUnknownEvent)?;

// 	let decoded = event.decode_log(&log_data, true).expect("Failed to decode log");

// 	let coerce_bytes = |(bytes, _): (&[u8], usize)| {
// 		let mut array = [0u8; 32];
// 		array.copy_from_slice(bytes);
// 		array
// 	};

// 	if let Some(selector) = decoded.selector {
// 		match selector {
// 			//TODO: Not sure all these fields are actually indexed
// 			COUNTERPARTY_LOCKED_SELECT => {
// 				let bridge_transfer_id =
// 					decoded.indexed[0].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed("BridgeTransferId".to_string())
// 					})?;
// 				let initiator_address =
// 					decoded.indexed[1].as_fixed_bytes().map(coerce_bytes).ok_or_else(|| {
// 						BridgeContractError::ConversionFailed("InitiatorAddress".to_string())
// 					})?;
// 				let recipient_address = decoded.indexed[1].as_address().ok_or_else(|| {
// 					BridgeContractError::ConversionFailed("RecipientAddress".to_string())
// 				})?;
// 				let amount = decoded.indexed[2]
// 					.as_uint()
// 					.map(|(u, _)| u.into())
// 					.ok_or_else(|| BridgeContractError::ConversionFailed("Amount".to_string()))?;
// 				let hash_lock = decoded.indexed[3]
// 					.as_fixed_bytes()
// 					.map(coerce_bytes)
// 					.ok_or_else(|| BridgeContractError::ConversionFailed("HashLock".to_string()))?;
// 				let time_lock: TimeLock = decoded.indexed[4]
// 					.as_uint()
// 					.map(|(u, _)| u.into())
// 					.ok_or_else(|| BridgeContractError::ConversionFailed("TimeLock".to_string()))?;
// 				Ok(BridgeContractEvent::Locked(LockDetails {
// 					bridge_transfer_id: BridgeTransferId(bridge_transfer_id),
// 					initiator_address: BridgeAddress(initiator_address.to_vec()),
// 					recipient_address: BridgeAddress(EthAddress(recipient_address)),
// 					amount: Amount(amount),
// 					hash_lock: HashLock(hash_lock),
// 					time_lock,
// 				}))
// 			}
// 			COUNTERPARTY_COMPLETED_SELECT => {
// 				unimplemented!();
// 				// let bridge_transfer_id = decoded.indexed[0]
// 				// 	.as_fixed_bytes()
// 				// 	.map(coerce_bytes)
// 				// 	.ok_or_else(|| anyhow::anyhow!("Failed to decode BridgeTransferId"))?;
// 				// let pre_image = decoded.indexed[1]
// 				// 	.as_fixed_bytes()
// 				// 	.map(coerce_bytes)
// 				// 	.ok_or_else(|| anyhow::anyhow!("Failed to decode PreImage"))?;
// 				// Ok(BridgeContractCounterpartyEvent::Completed(CounterpartyCompletedDetails {}))
// 			}
// 			_ => {
// 				tracing::error!("Unknown event selector: {:x}", selector);
// 				Err(BridgeContractError::ConversionFailed("event selector".to_string()))
// 			}
// 		}
// 	} else {
// 		tracing::error!("Failed to decode event selector");
// 		Err(BridgeContractError::ConversionFailed("event selector".to_string()))
// 	}
// }
