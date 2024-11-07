use super::types::EthAddress;
use crate::chains::ethereum::types::AtomicBridgeCounterpartyMOVE;
use crate::chains::ethereum::types::AtomicBridgeInitiatorMOVE;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy_network::EthereumWallet;
use bridge_config::common::eth::EthConfig;
use bridge_util::chains::bridge_contracts::BridgeContractError;
use bridge_util::chains::bridge_contracts::BridgeContractEvent;
use bridge_util::chains::bridge_contracts::BridgeContractMonitoring;
use bridge_util::chains::bridge_contracts::BridgeContractResult;
use bridge_util::types::HashLockPreImage;
use bridge_util::types::LockDetails;
use bridge_util::types::{BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock};
use futures::SinkExt;
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use std::{pin::Pin, task::Poll};
use tokio::sync::{mpsc, oneshot};

pub struct EthMonitoring {
	listener: UnboundedReceiver<BridgeContractResult<BridgeContractEvent<EthAddress>>>,
}

impl BridgeContractMonitoring for EthMonitoring {
	type Address = EthAddress;
}

impl EthMonitoring {
	pub async fn build(
		config: &EthConfig,
		mut health_check_rx: mpsc::Receiver<oneshot::Sender<bool>>,
	) -> Result<Self, anyhow::Error> {
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
					config.eth_initiator_contract.parse().unwrap(), //If unwrap start fail. Config must be updated.
					rpc_provider.clone(),
				);
				let counterpart_contract = AtomicBridgeCounterpartyMOVE::new(
					config.eth_counterparty_contract.parse().unwrap(), //If unwrap start fail. Config must be updated.
					rpc_provider.clone(),
				);
				let mut last_processed_block = 0;
				loop {
					//Check if there's a health check request
					match health_check_rx.try_recv() {
						Ok(tx) => {
							if let Err(err) = tx.send(true) {
								tracing::warn!(
									"Eth Health check send on oneshot channel failed:{err}"
								);
							}
						}
						Err(mpsc::error::TryRecvError::Empty) => (), //nothing
						Err(err) => {
							tracing::warn!("Check Eth monitoring loop health channel error: {err}");
						}
					}

					//Get block number.
					let block_number = match tokio::time::timeout(
						tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
						rpc_provider.get_block_number(),
					)
					.await
					{
						Ok(Ok(block_number)) => block_number,
						Ok(Err(err)) => {
							if sender
								.send(Err(BridgeContractError::OnChainError(format!(
									"Eth get blocknumber request failed: {err}"
								))))
								.await
								.is_err()
							{
								tracing::error!("Failed to send event to listener channel");
								break;
							}
							let _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
							continue;
						}
						Err(err) => {
							if sender
								.send(Err(BridgeContractError::OnChainError(format!(
									"Eth get blocknumber timeout: {err}"
								))))
								.await
								.is_err()
							{
								tracing::error!("Failed to send event to listener channel");
								break;
							}
							let _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
							continue;
						}
					};
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
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
							initiator_initiate_event_filter.query(),
						)
						.await
						{
							Ok(Ok(events)) => {
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
							Ok(Err(_)) => {
								if sender
									.send(Err(BridgeContractError::OnChainError("Eth monitoring query initiator_initiate_event_filter timeout.".to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
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
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),initiator_trcompleted_event_filter.query()).await {
							Ok(Ok(events)) => {
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
							Ok(Err(_)) => {
								if sender
									.send(Err(BridgeContractError::OnChainError("Eth monitoring query initiator_trcompleted_event_filter timeout.".to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
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
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
							initiator_trrefund_event_filter.query(),
						)
						.await
						{
							Ok(Ok(events)) => {
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
							Ok(Err(_)) => {
								if sender
									.send(Err(BridgeContractError::OnChainError("Eth monitoring query initiator_trrefund_event_filter timeout.".to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
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
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
							counterpart_trlocked_event_filter.query(),
						)
						.await
						{
							Ok(Ok(events)) => {
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
							Ok(Err(_)) => {
								if sender
									.send(Err(BridgeContractError::OnChainError("Eth monitoring query counterpart_trlocked_event_filter timeout.".to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
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
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
							counterpart_trcompleted_event_filter.query(),
						)
						.await
						{
							Ok(Ok(events)) => {
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
							Ok(Err(_)) => {
								if sender
									.send(Err(BridgeContractError::OnChainError("Eth monitoring query counterpart_trcompleted_event_filter timeout.".to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
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
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
							counterpart_trcaborted_event_filter.query(),
						)
						.await
						{
							Ok(Ok(events)) => {
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
							Ok(Err(_)) => {
								if sender
									.send(Err(BridgeContractError::OnChainError("Eth monitoring query counterpart_trcaborted_event_filter timeout.".to_string())))
									.await
									.is_err()
								{
									tracing::error!("Failed to send event to listener channel");
									break;
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
					} // end match

					let _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
				} // end loop
			} // End spawn
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
