use crate::aptos::primitive_types::AptosStorage;
use aptos_api_types::{Address, HexEncodedBytes, MoveModuleBytecode, MoveResource};
use aptos_sdk::rest_client::Account;
use reth_primitives::Bytes;
use sov_modules_api::{StateMapAccessor, WorkingSet};
use sov_state::codec::BcsCodec;
use std::convert::Infallible;

use super::{AccountInfo, DbAccount};

/// The Aptos Database structure for storing and working with accounts and their modules.
pub(crate) struct AptosDb<'a, S: sov_modules_api::Spec> {
	/// Accounts storage
	pub(crate) accounts: sov_modules_api::StateMap<Address, DbAccount, BcsCodec>,
	/// Code storage
	/// Where K is the public key or `authentication_key` of the account
	pub(crate) code: sov_modules_api::StateMap<HexEncodedBytes, Bytes, BcsCodec>,
	/// Working set for the current transaction
	pub(crate) working_set: &'a mut WorkingSet<S>,
}

impl<'a, S: sov_modules_api::Spec> AptosDb<'a, S> {
	pub(crate) fn new(
		accounts: sov_modules_api::StateMap<Address, DbAccount, BcsCodec>,
		code: sov_modules_api::StateMap<HexEncodedBytes, Bytes, BcsCodec>,
		working_set: &'a mut WorkingSet<S>,
	) -> Self {
		Self { accounts, code, working_set }
	}
}

impl<'a, S: sov_modules_api::Spec> AptosStorage for AptosDb<'a, S> {
	type Error = Infallible;

	fn account(&mut self, account: Account) -> Result<AccountInfo, Self::Error> {
		let db_account = self
			.accounts
			.get(&Address::from(account.authentication_key.account_address()), self.working_set)
			.expect("Account not found");

		Ok(db_account.info)
	}

	fn resources(&mut self, account: Account) -> Result<Vec<MoveResource>, Self::Error> {
		// TODO move to new_raw_with_hash for better performance
		// let bytecode =
		// 	Bytecode::new_raw(self.code.get(&code_hash, self.working_set).unwrap_or_default());
		//
		// Ok(bytecode)
		todo!("resources not yet implemented")
	}

	fn modules(&mut self, account: Account) -> Result<Vec<MoveModuleBytecode>, Self::Error> {
		// let storage_value: U256 = if let Some(acc) = self.accounts.get(&address, self.working_set) {
		// 	acc.storage.get(&index, self.working_set).unwrap_or_default()
		// } else {
		// 	U256::default()
		// };
		//
		// Ok(storage_value)
		todo!("modules not yet implemented")
	}
}
