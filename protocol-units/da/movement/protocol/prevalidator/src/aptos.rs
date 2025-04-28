//! Prevalidation of Aptos transactions.

use crate::{Error, Prevalidated};

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
	pub fn prevalidate(
		&self,
		transaction: Transaction,
	) -> Result<Prevalidated<Transaction>, Error> {
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

		Ok(Prevalidated(transaction))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use aptos_crypto::{ed25519::Ed25519PrivateKey, Uniform as _};
	use aptos_sdk::{
		transaction_builder::TransactionFactory,
		types::{chain_id::ChainId, LocalAccount},
	};
	use aptos_types::account_config::aptos_test_root_address;
	use aptos_types::transaction::{RawTransaction, SignedTransaction};
	use movement_types::transaction::Transaction;

	use rand::rngs::OsRng;

	fn create_test_transaction(account: &LocalAccount) -> Result<Transaction, anyhow::Error> {
		let tx_factory = TransactionFactory::new(ChainId::test())
			.with_gas_unit_price(100)
			.with_max_gas_amount(100_000);

		let aptos_transaction = account
			.sign_with_transaction_builder(tx_factory.create_user_account(account.public_key()));

		let serialized_aptos_transaction = bcs::to_bytes(&aptos_transaction)?;

		Ok(Transaction::new(serialized_aptos_transaction, 0, aptos_transaction.sequence_number()))
	}

	fn create_test_transaction_with_invalid_signature(
		account: &LocalAccount,
	) -> Result<Transaction, anyhow::Error> {
		// Create a raw transaction
		let factory = TransactionFactory::new(ChainId::test());
		let raw_txn: RawTransaction = factory
			.transfer(aptos_test_root_address(), 1000)
			.sender(account.address())
			.sequence_number(0)
			.build();

		// Now generate a DIFFERENT key to sign (invalid signer)
		let bad_key = Ed25519PrivateKey::generate(&mut OsRng);

		// Manually create a SignedTransaction with the wrong signature
		let aptos_transaction = raw_txn.sign(
			&bad_key,
			account.public_key().clone(), // <- NOTE: still using the correct pubkey
		)?;
		let aptos_transaction: SignedTransaction = aptos_transaction.into_inner();

		let serialized_aptos_transaction = bcs::to_bytes(&aptos_transaction)?;

		Ok(Transaction::new(serialized_aptos_transaction, 0, aptos_transaction.sequence_number()))
	}

	#[test]
	fn invalid_transaction() -> Result<(), anyhow::Error> {
		let tx = Transaction::new(vec![42; 42], 0, 0);

		let validator = Validator::new();

		match validator.prevalidate(tx) {
			Err(Error::Validation(_)) => Ok(()),
			Err(e) => panic!("unexpected error: {e:?}"),
			Ok(_) => panic!("should not prevalidate an invalid payload"),
		}
	}

	#[test]
	fn incorrectly_signed_transaction() -> Result<(), anyhow::Error> {
		let account = LocalAccount::generate(&mut OsRng);
		let tx = create_test_transaction_with_invalid_signature(&account)?;

		let validator = Validator::new();

		match validator.prevalidate(tx) {
			Err(Error::Validation(_)) => Ok(()),
			Err(e) => panic!("unexpected error: {e:?}"),
			Ok(_) => panic!("should not prevalidate an invalid payload"),
		}
	}

	#[test]
	fn valid_transaction_no_whitelist() -> Result<(), anyhow::Error> {
		let account = LocalAccount::generate(&mut OsRng);
		let tx = create_test_transaction(&account)?;
		let tx_hash = tx.id();

		let validator = Validator::new();
		let Prevalidated(tx) = validator.prevalidate(tx)?;

		assert_eq!(tx.id(), tx_hash);
		Ok(())
	}

	#[test]
	fn valid_transaction_sender_whitelisted() -> Result<(), anyhow::Error> {
		let account = LocalAccount::generate(&mut OsRng);
		let tx = create_test_transaction(&account)?;
		let tx_hash = tx.id();

		let validator = Validator::with_whitelist([account.address()]);
		let Prevalidated(tx) = validator.prevalidate(tx)?;

		assert_eq!(tx.id(), tx_hash);
		Ok(())
	}

	#[test]
	fn valid_transaction_sender_not_in_whitelist() -> Result<(), anyhow::Error> {
		let mut rng = OsRng;
		let account = LocalAccount::generate(&mut rng);
		let tx = create_test_transaction(&account)?;
		let whitelisted_account = LocalAccount::generate(&mut rng);

		let validator = Validator::with_whitelist([whitelisted_account.address()]);

		match validator.prevalidate(tx) {
			Err(Error::Validation(_)) => Ok(()),
			Err(e) => panic!("unexpected error: {e:?}"),
			Ok(_) => panic!("should not prevalidate an invalid payload"),
		}
	}
}
