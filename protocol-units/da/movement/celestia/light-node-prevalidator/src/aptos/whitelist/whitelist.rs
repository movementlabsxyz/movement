use crate::{Error, Prevalidated, PrevalidatorOperations};
use aptos_types::{account_address::AccountAddress, transaction::Transaction as AptosTransaction};
use std::collections::HashSet;

pub struct Validator {
	whitelist: HashSet<AccountAddress>,
}

impl Validator {
	pub fn new(whitelist: HashSet<AccountAddress>) -> Self {
		Self { whitelist }
	}
}

#[tonic::async_trait]
impl PrevalidatorOperations<AptosTransaction, AptosTransaction> for Validator {
	/// Verifies a Transaction as a Valid AptosTransaction
	async fn prevalidate(
		&self,
		transaction: AptosTransaction,
	) -> Result<Prevalidated<AptosTransaction>, Error> {
		// reject all non-user transactions, check sender in whitelist for user transactions
		match &transaction {
			AptosTransaction::UserTransaction(user_transaction) => {
				if self.whitelist.contains(&user_transaction.sender()) {
					Ok(Prevalidated::new(transaction))
				} else {
					Err(Error::Validation("Sender not in whitelist".to_string()))
				}
			}
			_ => Err(Error::Validation("Invalid transaction type".to_string())),
		}
	}
}
