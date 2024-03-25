use super::db::AptosDb;
use crate::aptos::primitive_types::AptosStorageCommit;
use crate::aptos::DbAccount;
use aptos_api_types::Address;
use aptos_sdk::rest_client::Account;
use aptos_sdk::types::account_address::AccountAddress;
use revm::precompile::HashMap;
use sov_modules_api::StateMapAccessor;

impl<'a, S: sov_modules_api::Spec> AptosStorageCommit for AptosDb<'a, S> {
	fn commit(&mut self, changes: HashMap<AccountAddress, Account>) {
		for (address, account) in changes {
			let accounts_prefix = self.accounts.prefix();
			let mut db_acccount = self
				.accounts
				.get(&Address::from(account.authentication_key.account_address()), self.working_set)
				.unwrap_or_else(|| {
					DbAccount::new(
						accounts_prefix,
						Address::from(account.authentication_key.account_address()),
					)
				});
		}
	}
}
