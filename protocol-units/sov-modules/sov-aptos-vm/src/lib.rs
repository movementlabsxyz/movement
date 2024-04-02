mod aptos;
mod call;
mod event;
mod genesis;
mod helpers;
mod rpc;
mod signer;

pub use signer::DevSigner;

mod experimental {
	use super::aptos::DbAccount;
	use super::event::Event;
	use super::genesis::AptosConfig;
	use crate::aptos::primitive_types::{
		Receipt, SealedBlock, StateKeyWrapper, StateValueWrapper, TransactionSignedAndRecovered,
		ValidatorSignerWrapper,
	};
	use aptos_api_types::{HexEncodedBytes, MoveModuleBytecode, MoveResource};
	use aptos_config::config::{
		RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
		DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
	};
	use aptos_db::AptosDB;
	use aptos_storage_interface::DbReaderWriter;
	use sov_modules_api::{
		Context, DaSpec, Error, ModuleInfo, StateValue, StateValueAccessor, WorkingSet,
	};

	// @TODO: Check these vals. Make tracking issue.
	#[cfg(feature = "native")]
	pub(crate) const MIN_TRANSACTION_GAS: u64 = 21_000u64;
	#[cfg(feature = "native")]
	pub(crate) const MIN_CREATE_GAS: u64 = 53_000u64;

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	pub(crate) struct PendingTransaction {
		pub(crate) transaction: TransactionSignedAndRecovered,
		pub(crate) receipt: Receipt,
	}

	/// The sov-aptos module provides compatibility with the Aptos VM and Sovereign Labs
	#[allow(dead_code)]
	// #[cfg_attr(feature = "native", derive(sov_modules_api::ModuleCallJsonSchema))]
	#[derive(ModuleInfo, Clone)]
	pub struct SovAptosVM<S: sov_modules_api::Spec, Da: DaSpec> {
		#[address]
		pub(crate) address: S::Address,

		#[state]
		pub(crate) state_data: sov_modules_api::StateMap<StateKeyWrapper, StateValueWrapper>,

		#[state]
		pub(crate) db_path: StateValue<String>,

		// TODO: this may be redundant with address
		#[state]
		pub(crate) validator_signer: StateValue<Vec<u8>>, // TODO: fix validator signer incompatability

		#[state]
		pub(crate) genesis_hash: StateValue<Vec<u8>>, // TODO: fix genesis serialiation incompatability

		#[state]
		pub(crate) waypoint: StateValue<String>, // TODO: fix waypoint serialiation incompatability

		#[state]
		pub(crate) known_version: StateValue<u64>,

		#[state]
		pub(crate) chain_id: StateValue<u64>,
	}

	impl<S: sov_modules_api::Spec, Da: DaSpec> sov_modules_api::Module for SovAptosVM<S, Da> {
		type Spec = S;

		type Config = AptosConfig;

		type CallMessage = super::call::CallMessage;

		type Event = Event;

		fn genesis(
			&self,
			config: &Self::Config,
			working_set: &mut WorkingSet<S>,
		) -> Result<(), Error> {
			Ok(self.init_module(config, working_set)?)
		}

		fn call(
			&self,
			msg: Self::CallMessage,
			context: &Context<Self::Spec>,
			working_set: &mut WorkingSet<S>,
		) -> Result<sov_modules_api::CallResponse, Error> {
			Ok(self.execute_call(msg.serialized_txs, context, working_set)?)
		}
	}

	impl<S: sov_modules_api::Spec, Da: DaSpec> SovAptosVM<S, Da> {
		pub(crate) fn get_db(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<DbReaderWriter, Error> {
			let path = self
				.db_path
				.get(working_set)
				.ok_or(anyhow::Error::msg("Database path is not set."))?;

			let aptosdb = AptosDB::open(
				StorageDirPaths::from_path(path),
				false,
				NO_OP_STORAGE_PRUNER_CONFIG,
				RocksdbConfigs::default(),
				false, /* indexer */
				BUFFERED_STATE_TARGET_ITEMS,
				DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
			)?;
			Ok(DbReaderWriter::new(aptosdb))
		}

		pub(crate) fn get_validator_signer(
			&self,
			working_set: &mut WorkingSet<C::Storage>,
		) -> Result<ValidatorSignerWrapper, Error> {
			let serialized_validator_signer = self
				.validator_signer
				.get(working_set)
				.ok_or(anyhow::Error::msg("Validator signer is not set."))?;

			// TODO: seems redundant, but error types are different
			Ok(serde_json::from_slice::<ValidatorSignerWrapper>(&serialized_validator_signer)
				.expect("Failed to deserialize validator signer"))
		}
	}
}
