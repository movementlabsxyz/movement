use crate::{compiler::Compiler, ReleaseBundleError, ReleaseSigner};
use aptos_release_builder::aptos_framework_path;
use aptos_release_builder::components::consensus_config::generate_consensus_upgrade_proposal;
use aptos_sdk::types::transaction::{Script, SignedTransaction, TransactionPayload};
use aptos_types::on_chain_config::OnChainConsensusConfig;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

/// [Voter] can be used to wrap a proposal to prefix it with a gas upgrade.
pub struct Voter {
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: u32,
	pub framework_local_dir: Option<PathBuf>,
}

impl Voter {
	pub fn new(
		repo: &'static str,
		commit_hash: &'static str,
		bytecode_version: u32,
		framework_local_dir: Option<PathBuf>,
	) -> Self {
		Self { repo, commit_hash, bytecode_version, framework_local_dir }
	}

	pub fn head() -> Self {
		Self {
			repo: "doesn't matter",
			commit_hash: "doesn't matter",
			bytecode_version: 6,
			framework_local_dir: Some(aptos_framework_path()),
		}
	}

	/// Generates the bytecode for the gas upgrade proposal.
	pub fn vote_consensus_proposal_bytecode(&self) -> Result<Vec<u8>, ReleaseBundleError> {
		// generate the script
		let (_, update_gas_script) =
			generate_consensus_upgrade_proposal(&OnChainConsensusConfig::default(), true, vec![])
				.map_err(|e| ReleaseBundleError::Build(e.into()))?
				.pop()
				.map_or(Err(ReleaseBundleError::Build("no gas upgrade proposal".into())), Ok)?;

		let temp_dir = tempdir().map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let gas_script_path = temp_dir.path().join("proposal");
		let mut gas_script_path = gas_script_path.as_path().to_path_buf();
		gas_script_path.set_extension("move");
		fs::write(gas_script_path.as_path(), update_gas_script)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		// list all files in the temp dir
		let files =
			fs::read_dir(temp_dir.path()).map_err(|e| ReleaseBundleError::Build(e.into()))?;
		for file in files {
			let file = file.map_err(|e| ReleaseBundleError::Build(e.into()))?;
			println!("file: {:?}", file.path());
		}

		let compiler = Compiler::new(
			self.repo,
			self.commit_hash,
			self.bytecode_version,
			self.framework_local_dir.clone(),
		);

		let bytecode = compiler
			.compile_in_temp_dir_to_bytecode("proposal", &gas_script_path)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		Ok(bytecode)
	}

	/// Generate the transaction for the gas upgrade proposal.
	pub async fn vote_consensus_proposal_transaction(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<SignedTransaction, ReleaseBundleError> {
		let bytecode = self.vote_consensus_proposal_bytecode()?;
		let script_payload = TransactionPayload::Script(Script::new(bytecode, vec![], vec![]));

		// get the chain id
		let ledger_information = client
			.get_ledger_information()
			.await
			.map_err(|e| ReleaseBundleError::Proposing(Box::new(e)))?;
		let chain_id =
			aptos_types::chain_id::ChainId::new(ledger_information.into_inner().chain_id);

		let raw_transaction = aptos_types::transaction::RawTransaction::new(
			signer.release_account_address(client).await?,
			signer.release_account_sequence_number(client).await?,
			script_payload,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			chain_id,
		);
		let signed_transaction = signer.sign_release(raw_transaction).await?;

		Ok(signed_transaction)
	}

	pub async fn vote_consensus(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
		let signed_transaction = self
			.vote_consensus_proposal_transaction(
				signer,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				client,
			)
			.await?;

		let _response = client.submit_and_wait_bcs(&signed_transaction).await.map_err(|e| {
			ReleaseBundleError::Proposing(
				format!("failed to submit gas upgrade proposal: {:?}", e).into(),
			)
		})?;

		Ok(vec![signed_transaction])
	}
}
