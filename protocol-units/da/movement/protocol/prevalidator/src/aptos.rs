//! Prevalidation of Aptos transactions.

use crate::Error;

use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::SignedTransaction as AptosTransaction;
use movement_types::transaction::Transaction;

use std::collections::HashSet;

/// Prevalidates a Transaction as a correctly encoded and signed AptosTransaction,
/// optionally vetted against a whitelist of sender addresses.
pub struct Validator {
	whitelist: Option<HashSet<AccountAddress>>,
}

impl Validator {
	/// Creates a Validator with no whitelist. All well-formed signed transactions
	/// are validated.
	pub fn new() -> Self {
		Validator { whitelist: None }
	}

	/// Creates a Validator configured with a whitelist. Transactions are checked
	/// against the addresses in the whitelist, in addition to being well-formed
	/// and signed.
	pub fn with_whitelist<I>(whitelist: I) -> Self
	where
		I: IntoIterator<Item = AccountAddress>,
	{
		Validator { whitelist: Some(whitelist.into_iter().collect()) }
	}

	/// Returns `Ok` if the transaction is valid accordingly to this instance's
	/// configuration. `Err` is returned for validation errors.
	pub fn prevalidate(&self, transaction: &Transaction) -> Result<(), Error> {
		// Deserialize data as Aptos transaction, fail if invalid.
		let aptos_transaction: AptosTransaction =
			bcs::from_bytes(&transaction.data()).map_err(|e| {
				Error::Validation(format!("failed to deserialize Aptos transaction: {}", e))
			})?;

		// Verify that the signature is valid
		aptos_transaction
			.verify_signature()
			.map_err(|e| Error::Validation(format!("signature verification failed: {}", e)))?;

		// Check the sender against the whitelist, if provided.
		if let Some(whitelist) = &self.whitelist {
			if !whitelist.contains(&aptos_transaction.sender()) {
				return Err(Error::Validation("transaction sender not in whitelist".into()));
			}
		}

		Ok(())
	}
}
