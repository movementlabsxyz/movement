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
use dashmap::DashMap;
use futures::channel::mpsc;
use std::sync::Arc;
use thiserror::Error;

use crate::shared::testing::rng::RngSeededClone;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum MethodName {
	InitiateBridgeTransfer,
	CompleteBridgeTransfer,
	RefundBridgeTransfer,
	GetBridgeTransferDetails,
	LockBridgeTransferAssets,
	AbortBridgeTransfer,
}

use super::{CounterpartyCall, InitiatorCall, Transaction};

#[derive(Debug, Error, Clone)]
pub enum AbstractBlockchainClientError {
	#[error("Send error")]
	SendError,
	#[error("Random failure")]
	RandomFailure,
}

#[derive(Clone, Debug)]
pub enum ErrorConfig {
	None,
	InitiatorError(BridgeContractInitiatorError),
	CounterpartyError(BridgeContractCounterpartyError),
	CustomError(AbstractBlockchainClientError),
}

#[derive(Debug, Clone)]
pub struct CallConfig {
	pub error: ErrorConfig,
	pub delay: Option<std::time::Duration>,
}

impl Default for CallConfig {
	fn default() -> Self {
		Self { error: ErrorConfig::None, delay: None }
	}
}

impl<A, H, R> AbstractBlockchainClient<A, H, R>
where
	A: std::fmt::Debug,
	H: std::fmt::Debug,
	R: RngSeededClone,
{
	pub fn set_call_config(&mut self, method: MethodName, call_index: usize, config: CallConfig) {
		assert!(call_index > 0, "call_index must be greater than 0");
		if let Some(mut call_list) = self.call_configs.get_mut(&method) {
			if call_list.iter().any(|(idx, _)| *idx == call_index) {
				// Handle the case of duplicate entry here if needed
				panic!(
					"Duplicate entry found for method '{:?}' and call_index {}",
					method, call_index
				);
			} else {
				call_list.push((call_index, config));
			}
		} else {
			self.call_configs.entry(method).or_default().push((call_index, config));
		}
	}

	pub fn clear_call_configs(&mut self) {
		self.call_configs.clear();
	}

	fn register_call(&mut self, method: MethodName) {
		if let Some(mut call_list) = self.call_configs.get_mut(&method) {
			call_list.retain_mut(|(call_index, _)| {
				*call_index -= 1;
				*call_index > 0
			});
		}
	}

	fn have_call_config(&self, method: MethodName) -> Option<CallConfig> {
		self.call_configs.get(&method).and_then(|configs| {
			configs
				.iter()
				.find(|config| config.0 == 0)
				.map(|found_config| &found_config.1)
				.cloned()
		})
	}
}

#[derive(Clone)]
pub struct AbstractBlockchainClient<A, H, R> {
	pub transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
	pub rng: R,
	pub failure_rate: f64,
	pub false_positive_rate: f64,
	pub call_configs: Arc<DashMap<MethodName, Vec<(usize, CallConfig)>>>,
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
		Self {
			transaction_sender,
			rng,
			failure_rate,
			false_positive_rate,
			call_configs: Default::default(),
		}
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
		self.register_call(MethodName::InitiateBridgeTransfer);
		if let Some(config) = self.have_call_config(MethodName::InitiateBridgeTransfer) {
			if let Some(delay) = config.delay {
				tokio::time::sleep(delay).await;
			}
			match config.error {
				ErrorConfig::None => {}
				ErrorConfig::InitiatorError(e) => return Err(e),
				ErrorConfig::CounterpartyError(_) => {
					panic!("Unexpected CounterpartyError for Initiator method")
				}
				ErrorConfig::CustomError(e) => {
					return Err(BridgeContractInitiatorError::GenericError(e.to_string()))
				}
			}
		}
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

		self.register_call(MethodName::CompleteBridgeTransfer);
		if let Some(config) = self.have_call_config(MethodName::CompleteBridgeTransfer) {
			if let Some(delay) = config.delay {
				tokio::time::sleep(delay).await;
			}
			match config.error {
				ErrorConfig::None => {}
				ErrorConfig::InitiatorError(e) => return Err(e),
				ErrorConfig::CounterpartyError(_) => {
					panic!("Unexpected CounterpartyError for Initiator method")
				}
				ErrorConfig::CustomError(e) => {
					return Err(BridgeContractInitiatorError::GenericError(e.to_string()))
				}
			}
		}
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
		self.register_call(MethodName::LockBridgeTransferAssets);
		if let Some(config) = self.have_call_config(MethodName::LockBridgeTransferAssets) {
			if let Some(delay) = config.delay {
				tokio::time::sleep(delay).await;
			}
			match config.error {
				ErrorConfig::None => {}
				ErrorConfig::CounterpartyError(e) => return Err(e),
				ErrorConfig::InitiatorError(_) => {
					panic!("Unexpected InitiatorError for Counterparty method")
				}
				ErrorConfig::CustomError(e) => {
					return Err(BridgeContractCounterpartyError::GenericError(e.to_string()))
				}
			}
		}
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
		self.register_call(MethodName::CompleteBridgeTransfer);
		if let Some(config) = self.have_call_config(MethodName::CompleteBridgeTransfer) {
			if let Some(delay) = config.delay {
				tokio::time::sleep(delay).await;
			}
			match config.error {
				ErrorConfig::None => {}
				ErrorConfig::CounterpartyError(e) => return Err(e),
				ErrorConfig::InitiatorError(_) => {
					panic!("Unexpected InitiatorError for Counterparty method")
				}
				ErrorConfig::CustomError(e) => {
					return Err(BridgeContractCounterpartyError::GenericError(e.to_string()))
				}
			}
		}
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
