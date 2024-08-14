use bridge_shared::bridge_monitoring::BridgeContractCounterpartyMonitoring;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct MovementCounterpartyMonitoring<A, H> {
	listener: UnboundedReceiver<EthChainEvent<A, H>>,
	//ws: Roo<PubSubFrontend>,
}

impl BridgeContractCounterpartyMonitoring for MovementCounterpartyMonitoring<EthAddress, EthHash> {
	type Address = EthAddress;
	type Hash = EthHash;
}

impl EthInitiatorMonitoring<EthAddress, EthHash> {
	async fn run(
		rpc_url: &str,
		listener: UnboundedReceiver<EthChainEvent<EthAddress, EthHash>>,
	) -> Result<Self, anyhow::Error> {
		let ws = WsConnect::new(rpc_url);
		let ws = ProviderBuilder::new().on_ws(ws).await?;

		//TODO: this should be an arg
		let initiator_address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
		let filter = Filter::new()
			.address(initiator_address)
			.event("BridgeTransferInitiated(bytes32,address,bytes32,uint256)")
			.event("BridgeTransferCompleted(bytes32,bytes32)")
			.from_block(BlockNumberOrTag::Latest);

		let sub = ws.subscribe_logs(&filter).await?;
		let mut sub_stream = sub.into_stream();

		// Spawn a task to forward events to the listener channel
		let (sender, _) =
			tokio::sync::mpsc::unbounded_channel::<EthChainEvent<EthAddress, EthHash>>();

		tokio::spawn(async move {
			while let Some(log) = sub_stream.next().await {
				let event = decode_log_data(log)
					.map_err(|e| {
						tracing::error!("Failed to decode log data: {:?}", e);
					})
					.expect("Failed to decode log data");
				let event = EthChainEvent::InitiatorContractEvent(Ok(event.into()));
				if sender.send(event).is_err() {
					tracing::error!("Failed to send event to listener channel");
					break;
				}
			}
		});

		Ok(Self { listener, ws })
	}
}

impl Stream for EthInitiatorMonitoring<EthAddress, EthHash> {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(EthChainEvent::InitiatorContractEvent(contract_result))) =
			this.listener.poll_next_unpin(cx)
		{
			tracing::trace!(
				"InitiatorContractMonitoring: Received contract event: {:?}",
				contract_result
			);

			// Only listen to the initiator contract events
			match contract_result {
				Ok(contract_event) => match contract_event {
					SmartContractInitiatorEvent::InitiatedBridgeTransfer(details) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Initiated(details)))
					}
					SmartContractInitiatorEvent::CompletedBridgeTransfer(bridge_transfer_id) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Completed(
							bridge_transfer_id,
						)))
					}
				},
				Err(e) => {
					tracing::error!("Error in contract event: {:?}", e);
				}
			}
		}
		Poll::Pending
	}
}
