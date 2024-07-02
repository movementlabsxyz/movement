use async_trait::async_trait;
use bridge_shared::{
	bridge_contracts::{
		BridgeContractCounterparty, BridgeContractCounterpartyError,
		BridgeContractCounterpartyResult, BridgeContractInitiator, BridgeContractInitiatorError,
		BridgeContractInitiatorResult,
	},
	types::{
		Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId,
		HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock,
	},
};
use futures::channel::mpsc;

use crate::shared::testing::rng::RngSeededClone;

use super::{CounterpartyCall, InitiatorCall, Transaction};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AbstractBlockchainClientError {
	#[error("Send error")]
	SendError,
	#[error("Random failure")]
	RandomFailure,
}

#[derive(Clone)]
pub struct AbstractBlockchainClient<A, H, R> {
	pub transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
	pub rng: R,
	pub failure_rate: f64,
	pub false_positive_rate: f64,
}

impl<A, H, R> AbstractBlockchainClient<A, H, R>
where
	A: std::fmt::Debug,
	H: std::fmt::Debug,
	R: RngSeededClone,
{
	pub fn new(
		transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
		rng: R,
		failure_rate: f64,
		false_positive_rate: f64,
	) -> Self {
		Self { transaction_sender, rng, failure_rate, false_positive_rate }
	}

	pub fn send_transaction(
		&mut self,
		transaction: Transaction<A, H>,
	) -> Result<(), AbstractBlockchainClientError> {
		let random_value: f64 = self.rng.gen();

		if random_value < self.failure_rate {
			tracing::trace!("AbstractBlockchainClient: Sending RANDOM_FAILURE {:?}", transaction);
			return Err(AbstractBlockchainClientError::RandomFailure);
		}

		if random_value < self.false_positive_rate {
			tracing::trace!("AbstractBlockchainClient: Sending FALSE_POSITIVE {:?}", transaction);
			return Ok(());
		}

		tracing::trace!("AbstractBlockchainClient: Sending transaction: {:?}", transaction);
		self.transaction_sender
			.unbounded_send(transaction)
			.map_err(|_| AbstractBlockchainClientError::SendError)
	}
}

#[async_trait]
impl<A, H, R> BridgeContractInitiator for AbstractBlockchainClient<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType,
	R: RngSeededClone + Send + Sync + Unpin + Clone,
{
	type Address = A;
	type Hash = H;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractInitiatorResult<()> {
		let transaction = Transaction::Initiator(InitiatorCall::InitiateBridgeTransfer(
			initiator_address,
			recipient_address,
			amount,
			time_lock,
			hash_lock,
		));
		self.send_transaction(transaction)
			.map_err(BridgeContractInitiatorError::generic)
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractInitiatorResult<()> {
		let transaction = Transaction::Initiator(InitiatorCall::CompleteBridgeTransfer(
			bridge_transfer_id,
			secret,
		));

		self.send_transaction(transaction)
			.map_err(BridgeContractInitiatorError::generic)
	}

	async fn refund_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<()> {
		unimplemented!()
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		unimplemented!()
	}
}

#[async_trait]
impl<A, H, R> BridgeContractCounterparty for AbstractBlockchainClient<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType,
	R: RngSeededClone + Send + Sync + Unpin + Clone,
{
	type Address = A;
	type Hash = H;

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		recipient: RecipientAddress,
		amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {
		let transaction = Transaction::Counterparty(CounterpartyCall::LockBridgeTransfer(
			bridge_transfer_id,
			hash_lock,
			time_lock,
			recipient,
			amount,
		));
		self.send_transaction(transaction)
			.map_err(BridgeContractCounterpartyError::generic)
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		let transaction = Transaction::Counterparty(CounterpartyCall::CompleteBridgeTransfer(
			bridge_transfer_id,
			secret,
		));
		self.send_transaction(transaction)
			.map_err(BridgeContractCounterpartyError::generic)
	}

	async fn abort_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		unimplemented!()
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>
	{
		unimplemented!()
	}
}
