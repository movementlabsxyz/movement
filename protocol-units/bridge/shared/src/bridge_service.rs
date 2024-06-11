use futures::stream::StreamExt;
use futures::Stream;

use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{
		BridgeContractCounterpartyMonitoring, BridgeContractInitiatorEvent,
		BridgeContractInitiatorMonitoring,
	},
};

pub struct BlockchainService<A, H, BCC, BCCM, BCI, BCIM>
where
	BCC: BridgeContractCounterparty<A, H>,
	BCCM: BridgeContractCounterpartyMonitoring<A, H>,
	BCI: BridgeContractInitiator<A, H>,
	BCIM: BridgeContractInitiatorMonitoring<A, H>,
{
	pub initiator_contract: BCI,
	pub initiator_monitoring: BCIM,
	pub counter_party_contract: BCC,
	pub counter_party_monitoring: BCCM,

	_phantom: std::marker::PhantomData<(A, H)>,
}

impl<A, H, BCC, BCCM, BCI, BCIM> BlockchainService<A, H, BCC, BCCM, BCI, BCIM>
where
	BCC: BridgeContractCounterparty<A, H>,
	BCCM: BridgeContractCounterpartyMonitoring<A, H>,
	BCI: BridgeContractInitiator<A, H>,
	BCIM: BridgeContractInitiatorMonitoring<A, H>,
{
	pub fn build(
		initiator_contract: BCI,
		initiator_monitoring: BCIM,
		counter_party_contract: BCC,
		counter_party_monitoring: BCCM,
	) -> Self {
		Self {
			initiator_contract,
			initiator_monitoring,
			counter_party_contract,
			counter_party_monitoring,
			_phantom: std::marker::PhantomData,
		}
	}
}

// implement the Stream trait for the BlockchainService
// essentially polling the monitring streams for events
impl<A, H, BCC, BCCM, BCI, BCIM> Stream for BlockchainService<A, H, BCC, BCCM, BCI, BCIM>
where
	A: std::fmt::Debug + Unpin,
	H: std::fmt::Debug + Unpin,

	BCC: BridgeContractCounterparty<A, H> + Unpin,
	BCCM: BridgeContractCounterpartyMonitoring<A, H> + Unpin,
	BCI: BridgeContractInitiator<A, H> + Unpin,
	BCIM: BridgeContractInitiatorMonitoring<A, H> + Unpin,
{
	type Item = ();

	fn poll_next(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context,
	) -> std::task::Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// find streaming events from the initator monitoring
		if let std::task::Poll::Ready(Some(event)) = this.initiator_monitoring.poll_next_unpin(cx) {
			match event {
				// if the event is a bridge transfer initiated event
				// then lock the bridge transfer assets
				BridgeContractInitiatorEvent::BridgeTransferInitiated(transfer) => {
					tracing::debug!("Bridge transfer initiated: {:?}", transfer);
				}
				BridgeContractInitiatorEvent::BridgeTransferCompleted(_) => {
					tracing::debug!("Bridge transfer completed");
				}
				BridgeContractInitiatorEvent::BridgeTransferRefunded(_) => {
					tracing::debug!("Bridge transfer refunded");
				}
			}
		}

		std::task::Poll::Pending
	}
}
