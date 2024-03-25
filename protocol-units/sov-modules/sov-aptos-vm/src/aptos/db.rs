use crate::aptos::primitive_types::AptosStorage;
use aptos_api_types::{Address, HexEncodedBytes, MoveModule, MoveModuleBytecode, MoveResource};
use aptos_sdk::rest_client::Account;
use sov_modules_api::{StateMapAccessor, WorkingSet};
use sov_state::codec::BcsCodec;
use std::convert::Infallible;

use super::{AccountInfo, DbAccount};

/// The Aptos Database structure for storing and working with accounts and their modules.
pub(crate) struct AptosDb<'a, S: sov_modules_api::Spec> {
	/// Accounts storage
	pub(crate) accounts: sov_modules_api::StateMap<Address, DbAccount, BcsCodec>,
	/// Modules storage
	pub(crate) modules: sov_modules_api::StateMap<Address, Vec<MoveModule>, BcsCodec>,
	/// Resources storage
	pub(crate) resources: sov_modules_api::StateMap<Address, Vec<MoveResource>, BcsCodec>,
	/// Working set
	pub(crate) working_set: &'a mut WorkingSet<S>,
}

impl<'a, S: sov_modules_api::Spec> AptosDb<'a, S> {
	pub(crate) fn new(
		accounts: sov_modules_api::StateMap<Address, DbAccount, BcsCodec>,
		modules: sov_modules_api::StateMap<Address, Vec<MoveModule>, BcsCodec>,
		resources: sov_modules_api::StateMap<Address, Vec<MoveResource>, BcsCodec>,
		working_set: &'a mut WorkingSet<S>,
	) -> Self {
		Self { accounts, modules, resources, working_set }
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
		let resources = self
			.resources
			.get(&Address::from(account.authentication_key.account_address()), self.working_set)
			.expect("Modules not found");
		Ok(resources)
	}

	fn modules(&mut self, account: Account) -> Result<Vec<MoveModule>, Self::Error> {
		let modules = self
			.modules
			.get(&Address::from(account.authentication_key.account_address()), self.working_set)
			.expect("Modules not found");
		Ok(modules)
	}
}
