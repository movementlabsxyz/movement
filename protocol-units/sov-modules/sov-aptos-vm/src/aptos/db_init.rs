use aptos_api_types::{Address, HexEncodedBytes};
use reth_primitives::Bytes;
use sov_modules_api::StateMapAccessor;

use super::db::AptosDb;
use super::{AccountInfo, DbAccount};

/// Initializes database with a predefined account.
pub(crate) trait InitEvmDb {
	/// Inserts account information into the database.
	fn insert_account_info(&mut self, address: Address, acc: AccountInfo);
	/// Inserts code into the database. `HexEncodedBytes` is the hash of the account
	/// `authentication_key`, from `Account`. This is analogous to a public key.
	fn insert_code(&mut self, code_hash: HexEncodedBytes, code: Bytes);
}

impl<'a, S: sov_modules_api::Spec> InitEvmDb for AptosDb<'a, S> {
	fn insert_account_info(&mut self, sender: Address, info: AccountInfo) {
		let parent_prefix = self.accounts.prefix();
		let db_account = DbAccount::new_with_info(parent_prefix, sender, info);

		self.accounts.set(&sender, &db_account, self.working_set);
	}

	fn insert_code(&mut self, code_hash: HexEncodedBytes, code: Bytes) {
		self.code.set(&code_hash, &code, self.working_set);
	}
}
