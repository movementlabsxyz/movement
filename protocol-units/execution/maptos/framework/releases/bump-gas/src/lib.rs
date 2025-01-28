use aptos_framework::ReleaseBundle;
use aptos_gas_schedule::{AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule};
use aptos_release_builder::components::gas::generate_gas_upgrade_proposal;
use aptos_sdk::move_types::gas_algebra::GasQuantity;
use aptos_sdk::types::transaction::{
	RawTransaction, Script, SignedTransaction, Transaction, TransactionArgument, TransactionPayload,
};
use aptos_types::on_chain_config::GasScheduleV2;
use maptos_framework_release_util::{
	compiler::Compiler, Release, ReleaseBundleError, ReleaseSigner,
};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

/// [GasUpgrade] can be used to wrap a proposal to prefix it with a gas upgrade.
pub struct GasUpgrade<R>
where
	R: Release,
{
	pub wrapped_release: R,
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: u32,
	pub framework_local_dir: Option<PathBuf>,
	pub gas_schedule: GasScheduleV2,
}

impl<R> GasUpgrade<R>
where
	R: Release,
{
	pub fn new(
		wrapped_release: R,
		repo: &'static str,
		commit_hash: &'static str,
		bytecode_version: u32,
		framework_local_dir: Option<PathBuf>,
		gas_schedule: GasScheduleV2,
	) -> Self {
		Self {
			wrapped_release,
			repo,
			commit_hash,
			bytecode_version,
			framework_local_dir,
			gas_schedule,
		}
	}

	/// Generates the bytecode for the gas upgrade proposal.
	pub fn bump_gas_proposal_bytecode(&self) -> Result<Vec<u8>, ReleaseBundleError> {
		// generate the script
		let mut gas_parameters = AptosGasParameters::initial();
		gas_parameters.vm.txn.max_transaction_size_in_bytes = GasQuantity::new(100_000_000);

		let gas_schedule = aptos_types::on_chain_config::GasScheduleV2 {
			feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
			entries: gas_parameters
				.to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION),
		};

		let (_, update_gas_script) =
			generate_gas_upgrade_proposal(None, &gas_schedule, true, "".to_owned().into_bytes())
				.unwrap()
				.pop()
				.unwrap();

		let temp_dir = tempdir().map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let gas_script_path = temp_dir.path().join("gas_upgrade");
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
			.compile_in_temp_dir_to_bytecode("gas_upgrade", &gas_script_path)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		Ok(bytecode)
	}

	/// Generate the transaction for the gas upgrade proposal.
	pub async fn bump_gas_proposal_transaction(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: aptos_types::chain_id::ChainId,
	) -> Result<SignedTransaction, ReleaseBundleError> {
		let bytecode = self.bump_gas_proposal_bytecode()?;
		let script_payload = TransactionPayload::Script(Script::new(bytecode, vec![], vec![]));
		let raw_transaction = aptos_types::transaction::RawTransaction::new(
			signer.release_account_address().await?,
			start_sequence_number,
			script_payload,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			chain_id,
		);
		let signed_transaction = signer.sign_release(raw_transaction).await?;

		Ok(signed_transaction)
	}

	pub async fn bump_gas(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: aptos_types::chain_id::ChainId,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
		let signed_transaction = self
			.bump_gas_proposal_transaction(
				signer,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				chain_id,
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

impl<R> Release for GasUpgrade<R>
where
	R: Release,
{
	/// Note: the release bundle will not actual contain the gas upgrade proposal, so when running genesis with this release, the gas upgrade proposal will not be included.
	/// Instead you will need to use an OTA
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		self.wrapped_release.release_bundle()
	}

	async fn release(
		&self,
		signer: &impl maptos_framework_release_util::ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: aptos_types::chain_id::ChainId,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
		// generate and execute the gas upgrade proposal

		// run the wrapped release
		self.wrapped_release
			.release(
				signer,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				chain_id,
				client,
			)
			.await
	}
}
