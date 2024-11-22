use super::types::EthAddress;
use crate::chains::ethereum::types::NativeBridge;
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
use bridge_util::chains::bridge_contracts::BridgeTransferCompletedDetails;
use bridge_util::chains::bridge_contracts::BridgeTransferInitiatedDetails;
use bridge_util::types::Nonce;
use bridge_util::types::{BridgeAddress, BridgeTransferId};
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
				let native_contract = NativeBridge::new(
					config.eth_initiator_contract.parse().unwrap(), //If unwrap start fail. Config must be updated.
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
						let initiate_event_filter = native_contract
							.BridgeTransferInitiated_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						// event BridgeTransferCompleted(bytes32 indexed _bridgeTransferId, bytes32 pre_image);
						let completed_event_filter = native_contract
							.BridgeTransferCompleted_filter()
							.from_block(BlockNumberOrTag::Number(last_processed_block));
						//Initiator event stream
						match tokio::time::timeout(
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),
							initiate_event_filter.query(),
						)
						.await
						{
							Ok(Ok(events)) => {
								for (initiated, _log) in events {
									let event = {
										// BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
										let details: BridgeTransferInitiatedDetails<EthAddress> =
											BridgeTransferInitiatedDetails {
												bridge_transfer_id: BridgeTransferId(
													*initiated.bridgeTransferId,
												),
												initiator: BridgeAddress(EthAddress(
													Address::from(initiated.originator),
												)),
												recipient: BridgeAddress(
													initiated.recipient.to_vec(),
												),
												nonce: Nonce(initiated.nonce.wrapping_to::<u128>()),
												amount: initiated.amount.into(),
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
							tokio::time::Duration::from_secs(config.rest_connection_timeout_secs),completed_event_filter.query()).await {
							Ok(Ok(events)) => {
								for (completed, _log) in events {
									let event = {
										// BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
										let details: BridgeTransferCompletedDetails<EthAddress> =
											BridgeTransferCompletedDetails {
												bridge_transfer_id: BridgeTransferId(
													*completed.bridgeTransferId,
												),
												initiator: BridgeAddress(
													completed.originator.to_vec(),
												),
												recipient: BridgeAddress(EthAddress(
													Address::from(completed.recipient),
												)),
												nonce: bridge_util::types::Nonce(completed.nonce.wrapping_to::<u128>()),
												amount: completed.amount.into(),

											};
										BridgeContractEvent::Completed(details)
									};
									if sender.send(Ok(event)).await.is_err() {
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
					} // end if

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
