use super::Executor;
use aptos_api::Context;
use aptos_config::config::NodeConfig;
#[cfg(test)]
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{ed25519::Ed25519PublicKey, PrivateKey};
use aptos_db::AptosDB;
use aptos_executor::{
	block_executor::BlockExecutor,
	db_bootstrapper::{generate_waypoint, maybe_bootstrap},
};
use aptos_mempool::{core_mempool::CoreMempool, MempoolClientRequest, MempoolClientSender};
use aptos_sdk::types::on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig};
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
	chain_id::ChainId,
	transaction::{ChangeSet, Transaction, WriteSetPayload},
	validator_signer::ValidatorSigner,
};
use aptos_vm::AptosVM;
use aptos_vm_genesis::{
	default_gas_schedule, encode_genesis_change_set, GenesisConfiguration, TestValidator, Validator,
};
use maptos_execution_util::config::Config;

use anyhow::Context as _;
use futures::channel::mpsc as futures_mpsc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::RwLock as TokioRwLock;

#[cfg(test)]
use tempfile::TempDir;

use std::{path::PathBuf, sync::Arc};

impl Executor {
	pub fn genesis_change_set_and_validators(
		chain_id: ChainId,
		count: Option<usize>,
		public_key: &Ed25519PublicKey,
	) -> (ChangeSet, Vec<TestValidator>) {
		let framework = aptos_cached_packages::head_release_bundle();
		let test_validators = TestValidator::new_test_set(count, Some(100_000_000));
		let validators_: Vec<Validator> = test_validators.iter().map(|t| t.data.clone()).collect();
		let validators = &validators_;

		// This number should not exceed u64::MAX / 1_000_000_000
		// to avoid overflowing calculations in aptos-vm-genesis.
		// This will last several centuries.
		const EPOCH_DURATION_SECS: u64 = 60 * 60 * 24 * 1024 * 128;

		let genesis = encode_genesis_change_set(
			&public_key,
			validators,
			framework,
			chain_id,
			// todo: get this config from somewhere
			&GenesisConfiguration {
				allow_new_validators: true,
				epoch_duration_secs: EPOCH_DURATION_SECS,
				is_test: true,
				min_stake: 0,
				min_voting_threshold: 0,
				// 1M APTOS coins (with 8 decimals).
				max_stake: 100_000_000_000_000,
				recurring_lockup_duration_secs: EPOCH_DURATION_SECS * 2,
				required_proposer_stake: 0,
				rewards_apy_percentage: 0,
				voting_duration_secs: EPOCH_DURATION_SECS,
				voting_power_increase_limit: 50,
				employee_vesting_start: 1663456089,
				employee_vesting_period_duration: 5 * 60, // 5 minutes
				initial_features_override: None,
				randomness_config_override: None,
				jwk_consensus_config_override: None,
			},
			&OnChainConsensusConfig::default_for_genesis(),
			&OnChainExecutionConfig::default_for_genesis(),
			&default_gas_schedule(),
		);
		(genesis, test_validators)
	}

	/// Bootstrap a database with a genesis transaction if it is empty.
	pub fn maybe_bootstrap_empty_db(
		db_dir: &PathBuf,
		chain_id: ChainId,
		public_key: &Ed25519PublicKey,
	) -> Result<(DbReaderWriter, ValidatorSigner), anyhow::Error> {
		let db_rw = DbReaderWriter::new(AptosDB::new_for_test(db_dir));
		let (genesis, validators) =
			Self::genesis_change_set_and_validators(chain_id, Some(1), public_key);
		let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
		let validator_signer = ValidatorSigner::new(
			validators[0].data.owner_address,
			validators[0].consensus_key.clone(),
		);

		// check for context

		match db_rw.reader.get_latest_ledger_info_option()? {
			Some(ledger_info) => {
				// context exists
				tracing::info!("Ledger info found, not bootstrapping DB: {:?}", ledger_info);
			}
			None => {
				// context does not exist
				// simply continue
				tracing::info!("No ledger info found, bootstrapping DB.");
				let waypoint = generate_waypoint::<AptosVM>(&db_rw, &genesis_txn)?;
				maybe_bootstrap::<AptosVM>(&db_rw, &genesis_txn, waypoint)?
					.ok_or(anyhow::anyhow!("Failed to bootstrap DB"))?;
				assert!(db_rw.reader.get_latest_ledger_info_option()?.is_some());
			}
		}

		Ok((db_rw, validator_signer))
	}

	pub fn bootstrap(
		mempool_client_sender: MempoolClientSender,
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		node_config: NodeConfig,
		maptos_config: Config,
	) -> Result<Self, anyhow::Error> {
		let (db, signer) = Self::maybe_bootstrap_empty_db(
			maptos_config.chain.maptos_db_path.as_ref().context("No db path provided.")?,
			maptos_config.chain.maptos_chain_id.clone(),
			&maptos_config.chain.maptos_private_key.public_key(),
		)?;
		let reader = db.reader.clone();
		let core_mempool = Arc::new(StdRwLock::new(CoreMempool::new(&node_config)));

		Ok(Self {
			block_executor: Arc::new(BlockExecutor::new(db.clone())),
			db,
			signer,
			core_mempool,
			mempool_client_sender: mempool_client_sender.clone(),
			mempool_client_receiver: Arc::new(TokioRwLock::new(mempool_client_receiver)),
			node_config: node_config.clone(),
			context: Arc::new(Context::new(
				maptos_config.chain.maptos_chain_id.clone(),
				reader,
				mempool_client_sender,
				node_config,
				None,
			)),
			listen_url: format!(
				"{}:{}",
				maptos_config.chain.maptos_rest_listen_hostname,
				maptos_config.chain.maptos_rest_listen_port
			),
			maptos_config,
		})
	}

	pub fn try_from_config(maptos_config: &Config) -> Result<Self, anyhow::Error> {
		// use the default signer, block executor, and mempool
		let (mempool_client_sender, mempool_client_receiver) =
			futures_mpsc::channel::<MempoolClientRequest>(10);
		let mut node_config = NodeConfig::default();

		node_config.indexer.enabled = true;
		// indexer config
		node_config.indexer.processor = Some("default_processor".to_string());
		node_config.indexer.check_chain_id = Some(false);
		node_config.indexer.skip_migrations = Some(false);
		node_config.indexer.fetch_tasks = Some(4);
		node_config.indexer.processor_tasks = Some(4);
		node_config.indexer.emit_every = Some(4);
		node_config.indexer.batch_size = Some(8);
		node_config.indexer.gap_lookback_versions = Some(4);

		node_config.indexer_grpc.enabled = true;

		node_config.indexer.postgres_uri =
			Some("postgresql://postgres:password@localhost:5432".to_string());

		// indexer_grpc config
		node_config.indexer_grpc.processor_batch_size = 4;
		node_config.indexer_grpc.processor_task_count = 4;
		node_config.indexer_grpc.output_batch_size = 4;
		node_config.indexer_grpc.address = format!(
			"{}:{}",
			maptos_config.indexer.maptos_indexer_grpc_listen_hostname,
			maptos_config.indexer.maptos_indexer_grpc_listen_port
		)
		.parse()?;
		node_config.indexer_grpc.use_data_service_interface = true;

		// indexer table info config
		node_config.indexer_table_info.enabled = true;
		node_config.storage.dir = "./.movement/maptos-storage".to_string().into();
		node_config.storage.set_data_dir(node_config.storage.dir.clone());

		Self::bootstrap(
			mempool_client_sender,
			mempool_client_receiver,
			node_config,
			maptos_config.clone(),
		)
	}

	#[cfg(test)]
	pub fn try_test_default(
		private_key: Ed25519PrivateKey,
	) -> Result<(Self, TempDir), anyhow::Error> {
		let tempdir = tempfile::tempdir()?;

		let mut maptos_config = Config::default();
		maptos_config.chain.maptos_private_key = private_key;

		// replace the db path with the temporary directory
		maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
		let executor = Self::try_from_config(&maptos_config)?;
		Ok((executor, tempdir))
	}
}
