pub mod whitelist;
use crate::{
	aptos::transaction::Validator as AptosTransactionValidator, Error, Prevalidated,
	PrevalidatorOperations,
};
use movement_types::transaction::Transaction;

/// Prevalidates a Transaction as an AptosTransaction and one that is whitelisted.
pub struct Validator {
	whitelist_validator: whitelist::Validator,
}

impl Validator {
	/// Creates a new Validator with a whitelist of AccountAddresses.
	pub fn new(
		whitelist: std::collections::HashSet<aptos_types::account_address::AccountAddress>,
	) -> Self {
		Self { whitelist_validator: whitelist::Validator::new(whitelist) }
	}
}

#[tonic::async_trait]
impl PrevalidatorOperations<Transaction, Transaction> for Validator {
	/// Verifies a Transaction as a Valid Transaction
	async fn prevalidate(
		&self,
		transaction: Transaction,
	) -> Result<Prevalidated<Transaction>, Error> {
		let application_priority = transaction.application_priority();
		let sequence_number = transaction.sequence_number();

		let aptos_transaction = AptosTransactionValidator.prevalidate(transaction).await?;
		let aptos_transaction = self
			.whitelist_validator
			.prevalidate(aptos_transaction.into_inner())
			.await?
			.into_inner();

		Ok(Prevalidated(Transaction::new(
			bcs::to_bytes(&aptos_transaction).map_err(|e| {
				Error::Internal(format!("Failed to serialize AptosTransaction: {}", e))
			})?,
			application_priority,
			sequence_number,
		)))
	}
}
