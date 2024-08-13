use std::collections::HashMap;

use bridge_shared::types::{
	Amount, BridgeAddressType, BridgeHashType, GenUniqueHash, HashLockPreImage, RecipientAddress,
};
use event_logging::EthChainEvent;
use futures::{channel::mpsc, task::AtomicWaker};
use types::{EthTransaction, SmartContractCounterparty, SmartContractInitiator};

mod client;
mod event_logging;
mod types;
mod utils;

pub struct EthereumChain<A, H> {
	pub name: String,
	pub time: u64,
	pub accounts: HashMap<A, Amount>,
	pub events: Vec<EthChainEvent<A, H>>,

	pub initiator_contract: SmartContractInitiator<A, H>,
	pub counterparty_contract: SmartContractCounterparty<A, H>,

	pub transaction_sender: mpsc::UnboundedSender<EthTransaction<A, H>>,
	pub transaction_receiver: mpsc::UnboundedReceiver<EthTransaction<A, H>>,

	pub event_listeners: Vec<mpsc::UnboundedSender<EthTransaction<A, H>>>,

	waker: AtomicWaker,

	pub _phantom: std::marker::PhantomData<H>,
}

impl<A, H> EthereumChain<A, H>
where
	A: BridgeAddressType + From<RecipientAddress<A>>,
	H: BridgeHashType + GenUniqueHash,
	H: From<HashLockPreImage>,
{
	pub fn new(name: impl Into<String>) -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();
		let (event_sender, event_receiver) = mpsc::unbounded();
		let event_listeners = Vec::new();

		Self {
			name: name.into(),
			time: 0,
			accounts,
			events,
			initiator_contract: SmartContractInitiator::new(rng.seeded_clone()),
			counterparty_contract: SmartContractCounterparty::new(),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver,
			event_listeners,
			waker: AtomicWaker::new(),
			_phantom: std::marker::PhantomData,
		}
	}
}
