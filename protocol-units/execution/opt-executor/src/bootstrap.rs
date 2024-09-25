use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_db::AptosDB;
use aptos_executor::db_bootstrapper;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
	chain_id::ChainId,
	on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig},
	transaction::{ChangeSet, Transaction, WriteSetPayload},
	validator_signer::ValidatorSigner,
};
use aptos_vm::AptosVM;
use aptos_vm_genesis::{
	default_gas_schedule, encode_genesis_change_set, GenesisConfiguration, TestValidator, Validator,
};

use std::path::Path;

fn genesis_change_set_and_validators(
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
	db_dir: impl AsRef<Path> + Clone,
	chain_id: ChainId,
	public_key: &Ed25519PublicKey,
) -> Result<(DbReaderWriter, ValidatorSigner), anyhow::Error> {
	let db_rw = DbReaderWriter::new(AptosDB::new_for_test(db_dir));
	let (genesis, validators) = genesis_change_set_and_validators(chain_id, Some(1), public_key);
	let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
	let validator_signer =
		ValidatorSigner::new(validators[0].data.owner_address, validators[0].consensus_key.clone());

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
			let waypoint = db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, &genesis_txn)?;
			db_bootstrapper::maybe_bootstrap::<AptosVM>(&db_rw, &genesis_txn, waypoint)?
				.ok_or(anyhow::anyhow!("Failed to bootstrap DB"))?;
			assert!(db_rw.reader.get_latest_ledger_info_option()?.is_some());
		}
	}

	Ok((db_rw, validator_signer))
}
