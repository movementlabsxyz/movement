use anyhow::Result;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use aptos_vm_genesis::{test_genesis_change_set_and_validators, GENESIS_KEYPAIR};
use dirs;
use serde_json;
use std::fs;

use crate::experimental::SovAptosVM;
use aptos_types::transaction::{Transaction, WriteSetPayload};
use sov_modules_api::{DaSpec, StateValueAccessor, WorkingSet};

const VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const MOVE_DB_DIR: &str = ".sov-aptosvm-db";

impl<S: sov_modules_api::Spec, Da: DaSpec> SovAptosVM<S, Da> {
	pub(crate) fn init_module(
		&self,
		config: &<Self as sov_modules_api::Module>::Config,
		working_set: &mut WorkingSet<S>,
	) -> Result<()> {
		// get the validator signer
		let (genesis, validators) = test_genesis_change_set_and_validators(Some(1));
		let signer = ValidatorSigner::new(
			validators[0].data.owner_address,
			validators[0].consensus_key.clone(),
		);
		self.validator_signer.set(&serde_json::to_vec(&signer)?, working_set);

		// issue the gnesis transaction
		let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
		// 1. create the db
		let path = format!("{}/{}", dirs::home_dir().unwrap().to_str().unwrap(), MOVE_DB_DIR);
		if !fs::metadata(path.clone().as_str()).is_ok() {
			fs::create_dir_all(path.as_str()).unwrap();
		}
		// 2. store the db path
		self.db_path.set(&path, working_set);

		let db = self.get_db(working_set)?;

		// 3. write the genesis transaction
		let waypoint = generate_waypoint::<AptosVM>(&db, &genesis_txn)?;
		maybe_bootstrap::<AptosVM>(&db, &genesis_txn, waypoint)?;

		// set the genesis waypoint
		self.waypoint.set(&waypoint.to_string(), working_set);

		// set state version
		self.known_version.set(&0, working_set);

		drop(db); // need to drop the lock on the RocksDB
		  // set the genesis block
		let executor = self.get_executor(working_set)?;
		let genesis_block_id = executor.committed_block_id();
		println!("Genesis block id: {:?}", genesis_block_id);
		self.genesis_hash.set(&genesis_block_id.to_vec(), working_set);

		// might we need to commit the blocks first?
		/*executor.commit_blocks(
			vec![genesis_block_id],
			executor.
		)?;*/

		Ok(())
	}
}
