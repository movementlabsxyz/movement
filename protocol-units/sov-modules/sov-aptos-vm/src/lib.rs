// mod aptos;
mod call;
#[cfg(test)]
mod call_tests;
mod genesis;
mod rpc;
mod util;

// mod signer;

// pub use signer::DevSigner;

mod experimental {
	/*use crate::aptos::primitive_types::{
		EventWrapper, Receipt, SealedBlock, StateKeyWrapper, StateValueWrapper,
		TransactionSignedAndRecovered, ValidatorSignerWrapper,
	};*/
	// use aptos_api_types::{Event, HexEncodedBytes, MoveModuleBytecode, MoveResource};
	use aptos_config::config::{
		RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
		DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
	};
	use aptos_crypto::HashValue;
	use aptos_db::AptosDB;
	use aptos_executor::block_executor::BlockExecutor;
	use aptos_storage_interface::DbReaderWriter;
	use aptos_types::validator_signer::ValidatorSigner;
	use aptos_types::waypoint::Waypoint;
	use aptos_vm::AptosVM;
	use serde_json;
	use sov_modules_api::{
		Context, DaSpec, Error, ModuleInfo, StateMap, StateValue, StateValueAccessor, WorkingSet,
	};
	use std::str::FromStr;
	use std::path::PathBuf;
	// use aptos_mempool::core_mempool::{CoreMempool, TimelineState};
	use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
	use futures::{channel::mpsc as futures_mpsc, StreamExt};
	use aptos_config::config::NodeConfig;
	use aptos_types::chain_id::ChainId;

	// @TODO: Check these vals. Make tracking issue.
	#[cfg(feature = "native")]
	pub(crate) const MIN_TRANSACTION_GAS: u64 = 21_000u64;
	#[cfg(feature = "native")]
	pub(crate) const MIN_CREATE_GAS: u64 = 53_000u64;

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	pub(crate) struct PendingTransaction {

	}

	#[derive(Clone)]
	pub struct AptosVmConfig {
		pub data: Vec<u8>,
		pub path : PathBuf
	}

	/// The sov-aptos module provides compatibility with the Aptos VM
	#[allow(dead_code)]
	#[derive(ModuleInfo)]
	pub struct SovAptosVM<S: sov_modules_api::Spec> {
		#[address]
		pub(crate) address: S::Address,

		/*#[state]
		pub(crate) state_data: sov_modules_api::StateMap<StateKeyWrapper, StateValueWrapper>,*/

		#[state]
		pub(crate) db_path: StateValue<String>,

		// TODO: this may be redundant with address
		#[state]
		pub(crate) validator_signer: StateValue<Vec<u8>>, // TODO: fix validator signer incompatability

		// This is string because we are using transaction.hash: https://github.com/movemntdev/aptos-core/blob/112ad6d8e229a19cfe471153b2fd48f1f22b9684/crates/indexer/src/models/transactions.rs#L31
		// #[cfg(feature = "aptos-consensus")]
		#[state]
		pub(crate) transactions: StateMap<String, Vec<u8>>, // TODO: fix Transaction serialiation incompatability

		#[state]
		pub(crate) genesis_hash: StateValue<Vec<u8>>, // TODO: fix genesis serialiation incompatability

		#[state]
		pub(crate) waypoint: StateValue<String>, // TODO: fix waypoint serialiation incompatability

		#[state]
		pub(crate) known_version: StateValue<u64>,

		#[state]
		pub(crate) chain_id: StateValue<u8>,

	}

	impl<S: sov_modules_api::Spec> sov_modules_api::Module for SovAptosVM<S> {
		type Spec = S;

		type Config = AptosVmConfig;

		type CallMessage = super::call::CallMessage;

		type Event = ();

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
			_context: &Context<Self::Spec>,
			working_set: &mut WorkingSet<S>,
		) -> Result<sov_modules_api::CallResponse, Error> {
			Ok(self.execute_call(msg.serialized_txs, working_set)?)
		}
	}

	impl<S: sov_modules_api::Spec> SovAptosVM<S> {
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
			)
			.expect("Failed to open AptosDB");
			Ok(DbReaderWriter::new(aptosdb))
		}
		pub(crate) fn get_executor(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<BlockExecutor<AptosVM>, Error> {
			let db = self.get_db(working_set)?;
			Ok(BlockExecutor::new(db.clone()))
		}

		pub(crate) fn get_validator_signer(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<ValidatorSigner, Error> {
			let serialized_validator_signer = self
				.validator_signer
				.get(working_set)
				.ok_or(anyhow::Error::msg("Validator signer is not set."))?;
			Ok(serde_json::from_slice::<ValidatorSigner>(&serialized_validator_signer)
				.expect("Failed to deserialize validator signer"))
		}

		pub(crate) fn get_known_version(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<u64, Error> {
			let known_version = self
				.known_version
				.get(working_set)
				.ok_or(anyhow::Error::msg("Serialized waypoint hash is not set."))?;
			Ok(known_version)
		}

		pub(crate) fn get_waypoint(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<Waypoint, Error> {
			let serialized_waypoint = self
				.waypoint
				.get(working_set)
				.ok_or(anyhow::Error::msg("Serialized waypoint hash is not set."))?;
			println!("serialized_waypoint: {:?}", serialized_waypoint);

			// TODO: seems redundant, but error types are different
			Ok(Waypoint::from_str(serialized_waypoint.as_str())
				.expect("Failed to deserialize waypoint"))
		}

		pub(crate) fn get_genesis_hash(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<HashValue, Error> {
			let serialized_genesis_hash = self
				.genesis_hash
				.get(working_set)
				.ok_or(anyhow::Error::msg("Serialized genesis hash is not set."))?;

			// todo: remove expects
			Ok(HashValue::from_slice(serialized_genesis_hash)
				.expect("Failed to deserialize genesis hash"))
		}

		pub(crate) fn get_chain_id(
			&self,
			working_set: &mut WorkingSet<S>,
		) -> Result<ChainId, Error> {
			let chain_id = self
				.chain_id
				.get(working_set)
				.ok_or(anyhow::Error::msg("Chain ID is not set."))?;

			Ok(ChainId::new(chain_id))
		}

		pub(crate) fn get_aptos_api_context(
			&self,
			working_set: &mut WorkingSet<S>
		) -> Result<aptos_api::Context, Error> {

			let (mempool_client_sender, mut mempool_client_receiver) = futures_mpsc::channel::<MempoolClientRequest>(10);
			let db = self.get_db(working_set)?;
			let sender = MempoolClientSender::from(mempool_client_sender);
			let node_config = NodeConfig::default(); // todo: this will need to be modded
			let context = aptos_api::Context::new(
				self.get_chain_id(working_set)?.into(),
				db.reader.clone(),
				sender, node_config.clone(), 
				None // qiz: this may need to be an actual table reader
			);

			Ok(context)

		}

	}
}
