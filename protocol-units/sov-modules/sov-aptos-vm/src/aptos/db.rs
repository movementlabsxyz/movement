use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::state_store::errors::StateviewError;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::state_store::{state_value::StateValue as AptosStateValue, TStateView};
use aptos_types::transaction::Version;
use sov_modules_api::{StateMap, StateMapAccessor, WorkingSet};

type Result<T, E = StateviewError> = std::result::Result<T, E>;
/// The Aptos Database structure for storing and working with accounts and their modules.
pub(crate) struct SovAptosDb<'a, S: sov_modules_api::Spec> {
	pub(crate) state_kv_db: StateMap<Version, AptosStateValue>,
	/// Working set
	pub(crate) working_set: &'a mut WorkingSet<S>,
}

impl<'a, S: sov_modules_api::Spec> SovAptosDb<'a, S> {
	pub(crate) fn new(
		state_kv_db: StateMap<Version, AptosStateValue>,
		working_set: &'a mut WorkingSet<S>,
	) -> Self {
		Self { working_set, state_kv_db }
	}

	/// Get state view at `Version`, this is analogous to `blockheight`.
	pub(crate) fn state_view_at_version(
		&self,
		version: Option<Version>,
	) -> Result<DbStateView<'a, S>> {
		Ok(DbStateView { db: self.clone(), version, verify_against_state_root_hash: None })
	}
}

/// The `DbStateView` that is passed to the VM for transaction execution.
pub struct DbStateView<'a, S>
where
	S: sov_modules_api::Spec,
{
	db: &'a SovAptosDb<'a, S>,
	version: Option<Version>,
	verify_against_state_root_hash: Option<HashValue>,
}

impl<'a, S> DbStateView<'a, S>
where
	S: sov_modules_api::Spec,
{
	/// Get state value by key
	fn get(&self, key: &StateKey) -> Result<Option<AptosStateValue>> {
		Ok(if let Some(version) = self.version {
			if let Some(root_hash) = self.verify_against_state_root_hash {
				// We need to implement `get_state_value_with_proof_by_version` and use that here.
				// let (value, proof) = self.db.get_state_value_with_proof_by_version(key, version)?;
				// proof.verify(root_hash, CryptoHash::hash(key), value.as_ref())?;
				// value
				unimplemented!()
			} else {
				self.db.state_kv_db.get(version, self.working_set).get(key).cloned
			}
		} else {
			None
		})
	}
}

//`DbStateView` must implement `TStateView` trait for `AptosVM` to execute transactions.
impl<'a, S> TStateView for SovAptosDb<'a, S>
where
	S: sov_modules_api::Spec,
{
	type Key = StateKey;

	fn get_state_value(&self, state_key: &StateKey) -> Result<Option<AptosStateValue>> {
		self.get(state_key).map_err(Into::into)
	}

	fn get_usage(&self) -> Result<StateStorageUsage> {
		unimplemented!()
	}
}
