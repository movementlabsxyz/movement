use futures::channel::mpsc;

use crate::testing::rng::RngSeededClone;

use super::Transaction;

pub struct AbstractBlockchainClient<A, H, R> {
	pub transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
	pub rng: R,
	pub failure_rate: f64,
	pub false_positive_rate: f64,
}

impl<A, H, R> AbstractBlockchainClient<A, H, R>
where
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

	pub fn send_transaction(&mut self, transaction: Transaction<A, H>) -> Result<(), String> {
		let random_value: f64 = self.rng.gen();

		if random_value < self.failure_rate {
			return Err("Random failure occurred".to_string());
		}

		if random_value < self.false_positive_rate {
			// Not sending transaction, but thought it was send
			return Ok(());
		}

		self.transaction_sender
			.unbounded_send(transaction)
			.expect("Failed to send transaction");
		Ok(())
	}
}
