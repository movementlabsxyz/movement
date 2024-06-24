use futures::channel::mpsc;

use crate::shared::testing::rng::RngSeededClone;

use super::Transaction;

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
