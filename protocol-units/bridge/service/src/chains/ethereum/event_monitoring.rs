use super::types::EthAddress;
use crate::chains::bridge_contracts::BridgeContractError;
use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::chains::bridge_contracts::BridgeContractMonitoring;
use crate::chains::bridge_contracts::BridgeContractResult;
use crate::chains::ethereum::types::AtomicBridgeCounterparty;
use crate::chains::ethereum::types::AtomicBridgeInitiator;
use crate::types::Amount;
use crate::types::AssetType;
use crate::types::HashLockPreImage;
use crate::types::LockDetails;
use crate::types::{BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy_network::EthereumWallet;
use bridge_config::common::eth::EthConfig;
use futures::SinkExt;
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use std::{pin::Pin, task::Poll};
use tokio::select;

pub struct EthMonitoring {
	listener: UnboundedReceiver<BridgeContractResult<BridgeContractEvent<EthAddress>>>,
}

impl BridgeContractMonitoring for EthMonitoring {
	type Address = EthAddress;
}

impl EthMonitoring {
	pub async fn build(config: &EthConfig) -> Result<Self, anyhow::Error> {
		// let rpc_url = config.eth_ws_connection_url();
		// let ws = WsConnect::new(rpc_url);
		// let ws = ProviderBuilder::new().on_ws(ws).await?;
		// let initiator_contract =
		// 	AtomicBridgeInitiator::new(config.eth_initiator_contract.parse()?, ws.clone());

		let client_config: crate::chains::ethereum::client::Config = config.try_into()?;
		let rpc_provider = ProviderBuilder::new()
			.with_recommended_fillers()
			.wallet(EthereumWallet::from(client_config.signer_private_key.clone()))
			.on_builtin(client_config.rpc_url.as_str())
			.await?;
		let initiator_contract = AtomicBridgeInitiator::new(
			config.eth_initiator_contract.parse()?,
			rpc_provider.clone(),
		);

		tracing::info!(
			"Start Eth monitoring with initiator:{} counterpart:{}",
			config.eth_initiator_contract,
			config.eth_counterparty_contract
		);

		//register initiator event
		// event BridgeTransferInitiated(
		//     bytes32 indexed _bridgeTransferId,
		//     address indexed _originator,
		//     bytes32 indexed _recipient,
		//     uint256 amount,
		//     bytes32 _hashLock,
		//     uint256 _timeLock
		// );
		let initiator_initiate_event_filter = initiator_contract
			.BridgeTransferInitiated_filter()
			.from_block(BlockNumberOrTag::Latest)
			.watch()
			.await?;
		let mut initiator_initiate_sub_stream = initiator_initiate_event_filter.into_stream();

		// event BridgeTransferCompleted(bytes32 indexed _bridgeTransferId, bytes32 pre_image);
		let initiator_trcompleted_event_filter = initiator_contract
			.BridgeTransferCompleted_filter()
			.from_block(BlockNumberOrTag::Latest)
			.watch()
			.await?;
		let mut initiator_trcompleted_sub_stream = initiator_trcompleted_event_filter.into_stream();

		// event BridgeTransferRefunded(bytes32 indexed _bridgeTransferId);
		let initiator_trrefund_event_filter = initiator_contract
			.BridgeTransferRefunded_filter()
			.from_block(BlockNumberOrTag::Latest)
			.watch()
			.await?;
		let mut initiator_trrefund_sub_stream = initiator_trrefund_event_filter.into_stream();

		let counterpart_contract = AtomicBridgeCounterparty::new(
			config.eth_counterparty_contract.parse()?,
			rpc_provider.clone(),
		);
		//Register counterpart event
		// event BridgeTransferLocked(
		//     bytes32 indexed bridgeTransferId,
		//     bytes32 indexed initiator,
		//     address indexed recipient,
		//     uint256 amount,
		//     bytes32 hashLock,
		//     uint256 timeLock
		// );
		let counterpart_trlocked_event_filter = counterpart_contract
			.BridgeTransferLocked_filter()
			.from_block(BlockNumberOrTag::Latest)
			.watch()
			.await?;
		let mut counterpart_trlocked_sub_stream = counterpart_trlocked_event_filter.into_stream();

		//event BridgeTransferCompleted(bytes32 indexed bridgeTransferId, bytes32 pre_image);
		let counterpart_trcompleted_event_filter = counterpart_contract
			.BridgeTransferCompleted_filter()
			.from_block(BlockNumberOrTag::Latest)
			.watch()
			.await?;
		let mut counterpart_trcompleted_sub_stream =
			counterpart_trcompleted_event_filter.into_stream();

		//event BridgeTransferAborted(bytes32 indexed bridgeTransferId);
		let counterpart_trcaborted_event_filter = counterpart_contract
			.BridgeTransferCompleted_filter()
			.from_block(BlockNumberOrTag::Latest)
			.watch()
			.await?;
		let mut counterpart_trcaborted_sub_stream =
			counterpart_trcaborted_event_filter.into_stream();

		// Spawn a task to forward events to the listener channel
		let (mut sender, listener) = futures::channel::mpsc::unbounded::<
			BridgeContractResult<BridgeContractEvent<EthAddress>>,
		>();

		tokio::spawn(async move {
			loop {
				let event;
				select! {
					//Initiator event stream
					Some(res) = initiator_initiate_sub_stream.next() => {
						event = res.map(|(initiated, _log)| {
							// BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
							let details: BridgeTransferDetails<EthAddress> = BridgeTransferDetails {
								bridge_transfer_id: BridgeTransferId(*initiated._bridgeTransferId),
								initiator_address: BridgeAddress(EthAddress(Address::from(initiated._originator))),
								recipient_address: BridgeAddress(initiated._recipient.to_vec()),
								hash_lock: HashLock(*initiated._hashLock),
								time_lock: initiated._timeLock.into(),
								amount: initiated.amount.into(),
								state: 0,
							};
							BridgeContractEvent::Initiated(details)
						}).map_err(|err| BridgeContractError::OnChainError(err.to_string()));
					}
					Some(res) = initiator_trcompleted_sub_stream.next() => {
						event = res.map(|(completed, _log)| {
							BridgeContractEvent::InitialtorCompleted(BridgeTransferId(*completed._bridgeTransferId))
						}).map_err(|err| BridgeContractError::OnChainError(err.to_string()));
					}
					Some(res) = initiator_trrefund_sub_stream.next() => {
						event = res.map(|(refund, _log)| {
							BridgeContractEvent::Refunded(BridgeTransferId(*refund._bridgeTransferId))
						}).map_err(|err| BridgeContractError::OnChainError(err.to_string()));
					}
					//Counterpart event stream
					Some(res) = counterpart_trlocked_sub_stream.next() => {
						event = res.map(|(trlocked, _log)| {
							// BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
							let details: LockDetails<EthAddress> = LockDetails {
								bridge_transfer_id: BridgeTransferId(*trlocked.bridgeTransferId),
								initiator: BridgeAddress(trlocked.initiator.to_vec()),
								recipient: BridgeAddress(EthAddress(Address::from(trlocked.recipient))),
								amount: Amount(AssetType::Moveth(trlocked.amount.as_limbs()[0])),
								hash_lock: HashLock(*trlocked.hashLock),
								time_lock: trlocked.timeLock.into(),
							};
							BridgeContractEvent::Locked(details)
						}).map_err(|err| BridgeContractError::OnChainError(err.to_string()));
					}
					Some(res) = counterpart_trcompleted_sub_stream.next() => {
						event = res.map(|(completed, _log)| {
							BridgeContractEvent::CounterPartCompleted(BridgeTransferId(*completed.bridgeTransferId), HashLockPreImage(*completed.pre_image))
						}).map_err(|err| BridgeContractError::OnChainError(err.to_string()));
					}
					Some(res) = counterpart_trcaborted_sub_stream.next() => {
						event = res.map(|(aborted, _log)| {
							BridgeContractEvent::Cancelled(BridgeTransferId(*aborted.bridgeTransferId))
						}).map_err(|err| BridgeContractError::OnChainError(err.to_string()));
					}
				};
				if sender.send(event).await.is_err() {
					tracing::error!("Failed to send event to listener channel");
					break;
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
