use crate::experimental::SovAptosVM;
use anyhow::Result;
use aptos_bitvec::BitVec;
use aptos_consensus_types::block::Block;
use aptos_crypto::bls12381::Signature;
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::{HashValue, SigningKey};
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::aggregate_signature::{AggregateSignature, PartialSignatures};
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use aptos_types::block_info::BlockInfo;
use aptos_types::block_metadata::BlockMetadata;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::{
	generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures, LedgerInfoWithV0,
};
use aptos_types::transaction::Transaction;
use aptos_types::trusted_state::{TrustedState, TrustedStateChange};
use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier};
use chrono::Utc;
use poem_openapi::__private::serde_json;
use sov_modules_api::{
	CallResponse, Context, DaSpec, StateMapAccessor, StateValueAccessor, 
	WorkingSet,
};
use std::collections::BTreeMap;

use aptos_config::config::NodeConfig;

// qiz: How can the call message be a block? That is, how does this change how we submit transactions?
/// Aptos call message.
#[derive(
	borsh::BorshDeserialize,
	borsh::BorshSerialize,
	serde::Serialize,
	serde::Deserialize,
	Debug,
	PartialEq,
	Clone,
)]
pub struct CallMessage {
	pub serialized_txs: Vec<Vec<u8>>,
}

impl<S: sov_modules_api::Spec> SovAptosVM<S> {
	pub(crate) fn execute_call(
		&self,
		serialized_txs: Vec<Vec<u8>>,
		working_set: &mut WorkingSet<S>,
	) -> Result<CallResponse> {
		// timestamp
		let unix_now = Utc::now().timestamp() as u64;

		// get db for reference
		let db = self.get_db(working_set)?;

		// get the validator signer
		let validator_signer = self.get_validator_signer(working_set)?;

		// get the parent (genesis block)
		let parent_block_id = self.get_genesis_hash(working_set)?;

		// produce the block meta
		let latest_ledger_info = db.reader.get_latest_ledger_info()?;
		let next_epoch = latest_ledger_info.ledger_info().next_block_epoch();
		let block_id = HashValue::random();
		let block_meta = Transaction::BlockMetadata(BlockMetadata::new(
			block_id,
			next_epoch,
			0,
			validator_signer.author(),
			vec![],
			vec![],
			unix_now,
		));

		let mut txs = vec![];
		for serialized_tx in serialized_txs {
			let tx = serde_json::from_slice::<Transaction>(&serialized_tx)
				.expect("Failed to deserialize transaction");
			txs.push(tx.clone());
			let hash = tx.hash(); // diem crypto hasher
			let str_hash = hash.to_string();
			self.transactions.set(&str_hash, &serialized_tx, working_set);
		}

		// store the checkpoint
		let checkpoint = Transaction::StateCheckpoint(HashValue::random());

		// form the complete block
		let mut block = vec![];
		block.push(block_meta);
		block.extend(txs);
		// block.push(checkpoint);

		println!("BLOCK: {:?}", block);

		drop(db); // drop the db from above so that the executor can use RocksDB

		// execute the transaction in Aptos
		let executor = self.get_executor(working_set)?;
		// let parent_block_id = executor.committed_block_id();
		// Create a map from author to signatures.

		println!("EXECUTING BLOCK {:?} {:?}", block_id, parent_block_id);
		let result = executor.execute_block(
			(block_id, block).into(),
			parent_block_id,
			BlockExecutorConfigFromOnchain::new_no_block_limit(),
		)?;
		let chain_id = self.chain_id.get(working_set).unwrap();

		// sign for the the ledger
		// last three args are likely wrong, where to get this data.
		let ledger_info = LedgerInfo::new(
			BlockInfo::new(
				next_epoch,
				0,
				block_id,
				result.root_hash(),
				result.version(),
				unix_now,
				result.epoch_state().clone(),
			),
			HashValue::zero(),
		);

		println!("COMMITTING BLOCK: {:?} {:?}", block_id, parent_block_id);
		let li = generate_ledger_info_with_sig(&[validator_signer], ledger_info);
		executor
			.commit_blocks(vec![block_id], li.clone())
			.expect("Failed to commit blocks");

		// manage epoch an parent block id
		if li.ledger_info().ends_epoch() {
			let epoch_genesis_id =
				Block::make_genesis_block_from_ledger_info(li.ledger_info()).id();
			self.genesis_hash.set(&epoch_genesis_id.to_vec(), working_set);
		}

		drop(executor);
		// prove state
		let db_too = self.get_db(working_set)?;
		let state_proof = db_too.reader.get_state_proof(self.get_known_version(working_set)?)?;
		let trusted_state = TrustedState::from_epoch_waypoint(self.get_waypoint(working_set)?);
		let trusted_state = match trusted_state.verify_and_ratchet(&state_proof) {
			Ok(TrustedStateChange::Epoch { new_state, .. }) => new_state,
			_ => panic!("unexpected state change"),
		};
		self.waypoint.set(&trusted_state.waypoint().to_string(), working_set);
		self.known_version.set(&trusted_state.version(), working_set);

		// TODO: may want to use a lower level of execution abstraction
		// TODO: see https://github.com/movemntdev/aptos-core/blob/main/aptos-move/block-executor/src/executor.rs#L73
		// TODO: for an entrypoint that does not require a block.
		Ok(CallResponse::default())
	}
}
