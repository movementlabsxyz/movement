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
	pub(crate) state_data: StateMap<Version, AptosStateValue>,
	/// Working set
	pub(crate) working_set: &'a mut WorkingSet<S>,
}

impl<'a, S: sov_modules_api::Spec> SovAptosDb<'a, S> {
	pub(crate) fn new(
		state_data: StateMap<Version, AptosStateValue>,
		working_set: &'a mut WorkingSet<S>,
	) -> Self {
		Self { working_set, state_data }
	}

	/// Get state view at `Version`, this is analogous to `blockheight`.
	pub(crate) fn state_view_at_version(
		&self,
		version: Option<Version>,
	) -> Result<SovAptosDb<'a, S>> {
		Ok(SovAptosDb { state_data: self.state_data.clone(), working_set: self.working_set })
	}
}

//`DbStateView` must implement `TStateView` trait for `AptosVM` to execute transactions.
impl<'a, S> TStateView for SovAptosDb<'a, S>
where
	S: sov_modules_api::Spec,
{
	type Key = Version;

	fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<AptosStateValue>> {
		let state_value = self.state_data.get(state_key, self.working_set)?;
		Ok(Some(state_value))
	}

	fn get_usage(&self) -> Result<StateStorageUsage> {
		unimplemented!()
	}
}
