use crate::{Error, Prevalidated, PrevalidatorOperations};
use aptos_types::transaction::SignedTransaction as AptosTransaction;
use movement_types::transaction::Transaction;

pub struct Validator;

#[tonic::async_trait]
impl PrevalidatorOperations<Transaction, AptosTransaction> for Validator {
	/// Verifies a Transaction as a Valid AptosTransaction
	async fn prevalidate(
		&self,
		transaction: Transaction,
	) -> Result<Prevalidated<AptosTransaction>, Error> {
		// Only allow properly signed user transactions that can be deserialized from the transaction.data()
		let aptos_transaction: AptosTransaction =
			bcs::from_bytes(&transaction.data()).map_err(|e| {
				Error::Validation(format!("Failed to deserialize AptosTransaction: {}", e))
			})?;

		aptos_transaction
			.verify_signature()
			.map_err(|e| Error::Validation(format!("Failed to prevalidate signature: {}", e)))?;

		Ok(Prevalidated::new(aptos_transaction))
	}
}
