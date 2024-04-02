use crate::aptos::primitive_types::{StateKeyWrapper, StateValueWrapper};
use anyhow::Error;
use aptos_crypto::hash::CryptoHash;
use aptos_sdk::move_types::language_storage::{ModuleId, StructTag};
use aptos_sdk::move_types::metadata::Metadata;
use aptos_sdk::move_types::resolver::{ModuleResolver, ResourceResolver};
use aptos_sdk::move_types::value::MoveTypeLayout;
use aptos_types::access_path::AccessPath;
use aptos_types::account_address::AccountAddress;
use aptos_types::state_store::errors::StateviewError;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::state_store::state_value::StateValue;
use aptos_types::state_store::{state_value::StateValue as AptosStateValue, TStateView};
use bytes::Bytes;
use move_binary_format::file_format::CompiledModule;
use move_core_types::gas_schedule::{GasCarrier, InternalGasUnits};
use move_table_extension::{TableHandle, TableOperation, TableResolver};
use sov_modules_api::{StateMap, StateMapAccessor, WorkingSet};
use std::cell::RefCell;
use std::fmt::Debug;

type Result<T, E = StateviewError> = std::result::Result<T, E>;
/// The Aptos Database structure for storing and working with accounts and their modules.
pub(crate) struct SovAptosDb<'a, S: sov_modules_api::Spec> {
	pub(crate) state_data: StateMap<StateKeyWrapper, StateValueWrapper>,
	/// Working set
	pub(crate) working_set: RefCell<&'a mut WorkingSet<S>>,
}

impl<'a, S: sov_modules_api::Spec> SovAptosDb<'a, S> {
	pub(crate) fn new(
		state_data: StateMap<StateKeyWrapper, StateValueWrapper>,
		working_set: RefCell<&'a mut WorkingSet<S>>,
	) -> Self {
		Self { working_set, state_data }
	}
}

//`DbStateView` must implement `TStateView` trait for `AptosVM` to execute transactions.
impl<'a, S> TStateView for SovAptosDb<'a, S>
where
	S: sov_modules_api::Spec,
{
	type Key = StateKey;

	fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<AptosStateValue>> {
		let mut working_set = self.working_set.borrow_mut();
		let state_key_wrapper = StateKeyWrapper::new(state_key.clone());
		let state_value_wrapper = self.state_data.get(&state_key_wrapper, &mut working_set);
		match state_value_wrapper {
			Some(state_value_wrapper) => {
				let state_value = state_value_wrapper.into();
				Ok(Some(state_value))
			},
			None => Ok(None),
		}
	}

	fn get_usage(&self) -> Result<StateStorageUsage> {
		unimplemented!()
	}
}

impl<'a, S: sov_modules_api::Spec> ResourceResolver for SovAptosDb<'a, S> {
	type Error = anyhow::Error;

	// @TODO Currently, the metadata and layout are not used.
	fn get_resource_bytes_with_metadata_and_layout(
		&self,
		address: &AccountAddress,
		struct_tag: &StructTag,
		_metadata: &[Metadata],
		_layout: Option<&MoveTypeLayout>,
	) -> Result<(Option<Bytes>, usize), Error> {
		let ap = AccessPath::resource_access_path(*address, struct_tag.clone())
			.expect("Invalid access path.");

		let mut working_set = self.working_set.borrow_mut();

		match self
			.state_data
			.get(&StateKeyWrapper::new(StateKey::access_path(ap)), &mut working_set)
		{
			Some(val) => Ok((Some(Bytes::from(val)), 0)),
			None => Ok((None, 0)),
		}
	}
}

impl<'a, S: sov_modules_api::Spec> ModuleResolver for SovAptosDb<'a, S> {
	type Error = anyhow::Error;

	fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
		let module_bytes = match self.get_module(module_id) {
			Ok(Some(bytes)) => bytes,
			_ => return vec![],
		};
		let module = match CompiledModule::deserialize(&module_bytes) {
			Ok(module) => module,
			_ => return vec![],
		};
		module.metadata.into()
	}

	fn get_module(&self, id: &ModuleId) -> std::result::Result<Option<Bytes>, Self::Error> {
		let ap = AccessPath::from(id);
		let mut working_set = self.working_set.borrow_mut();
		match self
			.state_data
			.get(&StateKeyWrapper::new(StateKey::access_path(ap)), &mut working_set)
		{
			Some(val) => Ok(Some(Bytes::from(val))),
			None => Ok(None),
		}
	}
}

impl<'a, S: sov_modules_api::Spec> TableResolver for SovAptosDb<'a, S> {
	fn resolve_table_entry(
		&self,
		handle: &TableHandle,
		key: &[u8],
	) -> std::result::Result<Option<Vec<u8>>, anyhow::Error> {
		let mut working_set = self.working_set.borrow_mut();
		let account_address = handle.0;
		let ap = AccessPath::new(AccountAddress::from(account_address), key.to_vec());
		let state_key_wrapper = StateKeyWrapper::new(StateKey::access_path(ap));
		let state_value_wrapper = self.state_data.get(&state_key_wrapper, &mut working_set);
		match state_value_wrapper {
			Some(state_value_wrapper) => {
				let state_value: StateValue = state_value_wrapper.into();
				Ok(Some(state_value.into_bytes()))
			},
			None => Ok(None),
		}
	}

	fn operation_cost(
		&self,
		op: TableOperation,
		key_size: usize,
		val_size: usize,
	) -> InternalGasUnits<GasCarrier> {
		unimplemented!()
	}
}
