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

#[cfg(test)]
mod tests {
	use super::*;
	use aptos_api_types::{AccountResource, EventHandle, TableItem};
	use aptos_sdk::bcs;
	use sov_modules_api::{StateMap, WorkingSet};
	use sov_state::codec::BcsCodec;
	use std::collections::HashMap;

	#[test]
	fn test_account() {
		let mut working_set = WorkingSet::new();
		let mut accounts = StateMap::new(BcsCodec);
		let address = Address::from([0u8; 32]);
		let db_account = DbAccount {
			info: AccountInfo {
				sequence_number: 1,
				authentication_key: HexEncodedBytes::from([0u8; 32]),
			},
		};
		accounts.insert(address, db_account, &mut working_set);

		let mut aptos_db = AptosDb::new(
			accounts,
			StateMap::new(BcsCodec),
			StateMap::new(BcsCodec),
			&mut working_set,
		);
		let account =
			Account { authentication_key: HexEncodedBytes::from([0u8; 32]), sequence_number: 1 };
		let result = aptos_db.account(account).unwrap();
		assert_eq!(result.sequence_number, 1);
	}

	#[test]
	fn test_resources() {
		let mut working_set = WorkingSet::new();
		let mut resources = StateMap::new(BcsCodec);
		let address = Address::from([0u8; 32]);
		let move_resources = vec![
			MoveResource::new(
				AccountResource::struct_tag(),
				bcs::to_bytes(&AccountResource {
					authentication_key: HexEncodedBytes::from([0u8; 32]),
					sequence_number: 1,
					coin_register_events: EventHandle::new(0, 0),
				})
				.unwrap(),
			),
			MoveResource::new(
				TableItem::struct_tag(),
				bcs::to_bytes(&TableItem {
					key: HexEncodedBytes::from([0u8; 32]),
					value: HexEncodedBytes::from([0u8; 32]),
				})
				.unwrap(),
			),
		];
		resources.insert(address, move_resources, &mut working_set);

		let mut aptos_db = AptosDb::new(
			StateMap::new(BcsCodec),
			StateMap::new(BcsCodec),
			resources,
			&mut working_set,
		);
		let account =
			Account { authentication_key: HexEncodedBytes::from([0u8; 32]), sequence_number: 1 };
		let result = aptos_db.resources(account).unwrap();
		assert_eq!(result.len(), 2);
	}

	#[test]
	fn test_modules() {
		let mut working_set = WorkingSet::new();
		let mut modules = StateMap::new(BcsCodec);
		let address = Address::from([0u8; 32]);
		let move_modules =
			vec![MoveModule { bytecode: MoveModuleBytecode::new(vec![]), abi: None }];
		modules.insert(address, move_modules, &mut working_set);

		let mut aptos_db = AptosDb::new(
			StateMap::new(BcsCodec),
			modules,
			StateMap::new(BcsCodec),
			&mut working_set,
		);
		let account =
			Account { authentication_key: HexEncodedBytes::from([0u8; 32]), sequence_number: 1 };
		let result = aptos_db.modules(account).unwrap();
		assert_eq!(result.len(), 1);
	}
}
