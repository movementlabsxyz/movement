use anyhow::Context as _;
use aptos_api::Context;
use aptos_config::config::NodeConfig;
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

use super::Executor;
use futures::channel::mpsc as futures_mpsc;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

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

		let epoch_duration_secs = 60 * 60 * 24 * 1024 * 256; // several centuries
		let genesis = encode_genesis_change_set(
			&public_key,
			validators,
			framework,
			chain_id,
			// todo: get this config from somewhere
			&GenesisConfiguration {
				allow_new_validators: true,
				epoch_duration_secs: epoch_duration_secs,
				is_test: true,
				min_stake: 0,
				min_voting_threshold: 0,
				// 1M APTOS coins (with 8 decimals).
				max_stake: 100_000_000_000_000,
				recurring_lockup_duration_secs: epoch_duration_secs * 2,
				required_proposer_stake: 0,
				rewards_apy_percentage: 10,
				voting_duration_secs: epoch_duration_secs,
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
		maptos_config: &Config,
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
				tracing::info!(
					"Ledger info found, not bootstrapping DB: {:?}",
					ledger_info
				);
			},
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
		maptos_config: &Config,
	) -> Result<Self, anyhow::Error> {
		let (db, signer) = Self::maybe_bootstrap_empty_db(
			&maptos_config.chain.maptos_db_path.clone().context("No db path provided.")?,
			maptos_config.chain.maptos_chain_id.clone(),
			&maptos_config.chain.maptos_private_key.clone().public_key(),
			maptos_config
		)?;
		let reader = db.reader.clone();
		let core_mempool = Arc::new(RwLock::new(CoreMempool::new(&node_config)));

		Ok(Self {
			block_executor: Arc::new(RwLock::new(BlockExecutor::new(db.clone()))),
			db,
			signer,
			core_mempool,
			mempool_client_sender: mempool_client_sender.clone(),
			mempool_client_receiver: Arc::new(RwLock::new(mempool_client_receiver)),
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
			maptos_config : maptos_config.clone()
		})
	}

	pub fn try_from_config(maptos_config: &Config) -> Result<Self, anyhow::Error> {
		// use the default signer, block executor, and mempool
		let (mempool_client_sender, mempool_client_receiver) =
			futures_mpsc::channel::<MempoolClientRequest>(10);
		let node_config = NodeConfig::default();
		Self::bootstrap(mempool_client_sender, mempool_client_receiver, node_config, maptos_config)
	}

	pub fn try_test_default() -> Result<Self, anyhow::Error> {
		let mut maptos_config = Config::default();

		// replace the db path with a temporary directory
		let value = tempfile::tempdir()?.into_path(); // todo: this works because it's at the top level, but won't be cleaned up automatically
		maptos_config.chain.maptos_db_path.replace(value);
		Self::try_from_config(&maptos_config)
	}

}
