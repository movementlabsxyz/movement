use crate::aptos::primitive_types::AptosStorage;
use aptos_api_types::{Address, HexEncodedBytes, MoveModule, MoveModuleBytecode, MoveResource};
use aptos_crypto::HashValue;
use aptos_db::AptosDB;
use aptos_sdk::rest_client::Account;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_storage_interface::DbReader;
use aptos_types::state_store::errors::StateviewError;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::state_store::{state_value::StateValue as AptosStateValue, TStateView};
use aptos_types::transaction::Version;
use sov_modules_api::{StateMap, StateMapAccessor, StateValue, WorkingSet};
use std::sync::Arc;

type Result<T, E = StateviewError> = std::result::Result<T, E>;
/// The Aptos Database structure for storing and working with accounts and their modules.
pub(crate) struct SovAptosDb<'a, S: sov_modules_api::Spec> {
	pub(crate) state_kv_db: StateMap<Version, AptosStateValue>,
	/// Working set
	pub(crate) working_set: &'a mut WorkingSet<S>,
}

impl<'a, S: sov_modules_api::Spec> SovAptosDb<'a, S> {
	pub(crate) fn new(working_set: &'a mut WorkingSet<S>, db: StateValue<AptosDB>) -> Self {
		Self { working_set, db }
	}

	/// Get state view at `Version`, this is analogous to `blockheight`.
	/// `Version` is a type alias for `u64` in the `aptos_types` module.
	/// Source code: https://github.com/0xmovses/aptos-core/blob/bd1644729bc2598d9769fbf556797d5a4f51bf35/types/src/transaction/mod.rs#L77
	///
	/// For reading state from the SovAptosDb. We purposefully do not implement the Aptos native `DbReader` trait
	/// because it is `Send` and `Sync`. The sov_modules_api::StateMap is not `Send` and `Sync` so instead we add
	/// this custom method.
	pub(crate) fn state_view_at_version(
		&self,
		version: Option<Version>,
	) -> Result<DbStateView<'a, S>> {
		Ok(DbStateView { db: self.clone(), version, verify_against_state_root_hash: None })
	}
}

/// The DbStateView that is passed to the VM for transaction execution.
/// We don't use the Aptos native `DbStateView` because its `db` field is `Arc<dyn DbReader>`,
/// meaning it's a dynamically dispatched trait object, so unable to derive
/// serialization/deserialization. Instead, in our custom type we use a concrete type `SovAptosDb`.
pub struct DbStateView<'a, S>
where
	S: sov_modules_api::Spec,
{
	db: &'a SovAptosDb<'a, S>,
	version: Option<Version>,
	verify_against_state_root_hash: Option<HashValue>,
}

//`DbStateView` must implement `TStateView` trait for `AptosVM` to execute transactions.
impl<'a, S> TStateView for DbStateView<'a, S>
where
	S: sov_modules_api::Spec,
{
	type Key = StateKey;

	fn get_state_value(&self, state_key: &StateKey) -> Result<Option<AptosStateValue>> {
		self.get(state_key).map_err(Into::into)
	}

	fn get_usage(&self) -> Result<StateStorageUsage> {
		self.db.get_state_storage_usage(self.version).map_err(Into::into)
	}
}
