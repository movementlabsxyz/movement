use super::db::AptosDb;
use revm::primitives::{Account, Address, HashMap};
use revm::DatabaseCommit;

impl<'a, S: sov_modules_api::Spec> DatabaseCommit for AptosDb<'a, S> {
	fn commit(&mut self, _changes: HashMap<Address, Account>) {
		todo!()
	}
}
