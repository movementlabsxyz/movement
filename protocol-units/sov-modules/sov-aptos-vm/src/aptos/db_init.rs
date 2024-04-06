use aptos_api_types::{Address, HexEncodedBytes, MoveModule, MoveResource};
use sov_modules_api::StateMapAccessor;

use super::{AccountInfo, DbAccount};

/// Initializes database with a predefined account.
pub(crate) trait InitAptosDb {
	/// Inserts account information into the database.
	fn insert_account_info(&mut self, address: Address, acc: AccountInfo);
	/// Inserts resources into the database for an address.
	fn insert_resources(&mut self, address: Address, resources: Vec<MoveResource>);
	/// Inserts modules into the database for an address.
	fn insert_modules(&mut self, address: Address, modules: Vec<MoveModule>);
}
