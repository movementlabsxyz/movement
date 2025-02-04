use aptos_framework::ReleaseBundle;
use aptos_release_builder::components::feature_flags::{
	generate_feature_upgrade_proposal, Features,
};
use aptos_sdk::types::transaction::{Script, SignedTransaction, TransactionPayload};
use maptos_framework_release_util::{
	compiler::Compiler, Release, ReleaseBundleError, ReleaseSigner,
};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

/// [SetFeatureFlags] can be used to wrap a proposal to prefix it with a gas upgrade.
pub struct SetFeatureFlags<R>
where
	R: Release,
{
	pub wrapped_release: R,
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: u32,
	pub framework_local_dir: Option<PathBuf>,
	pub features: Features,
}

impl<R> SetFeatureFlags<R>
where
	R: Release,
{
	pub fn new(
		wrapped_release: R,
		repo: &'static str,
		commit_hash: &'static str,
		bytecode_version: u32,
		framework_local_dir: Option<PathBuf>,
		features: Features,
	) -> Self {
		Self { wrapped_release, repo, commit_hash, bytecode_version, framework_local_dir, features }
	}

	/// Generates the bytecode for the gas upgrade proposal.
	pub fn set_feature_flags_proposal_bytecode(&self) -> Result<Vec<u8>, ReleaseBundleError> {
		let (_, update_gas_script) =
			generate_feature_upgrade_proposal(&self.features, true, vec![])
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
	pub async fn set_feature_flags_proposal_transaction(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<SignedTransaction, ReleaseBundleError> {
		let bytecode = self.set_feature_flags_proposal_bytecode()?;
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

	pub async fn set_feature_flags(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
		let signed_transaction = self
			.set_feature_flags_proposal_transaction(
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

impl<R> Release for SetFeatureFlags<R>
where
	R: Release,
{
	/// Note: the release bundle will not actually contain the gas upgrade proposal, so when running genesis with this release, the gas upgrade proposal will not be included.
	/// Instead you will need to use an OTA
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		self.wrapped_release.release_bundle()
	}

	async fn propose_release(
		&self,
		signer: &impl maptos_framework_release_util::ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
		// run the wrapped release
		let transactions = self
			.wrapped_release
			.propose_release(
				signer,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_sec_offset,
				client,
			)
			.await?;

		// generate and execute the gas upgrade proposal
		let now_u64 = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?
			.as_secs();
		let expiration_timestamp_secs = now_u64 + expiration_timestamp_sec_offset;
		self.set_feature_flags(
			signer,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			client,
		)
		.await?;

		Ok(transactions)
	}
}

#[macro_export]
// Macro definition
macro_rules! generate_feature_upgrade_module {
	($mod_name:ident, $struct_name:ident, $features_stanza:expr) => {
		pub mod $mod_name {
			use aptos_framework::ReleaseBundle;
			use aptos_framework_set_feature_flags_release::SetFeatureFlags;
			use aptos_release_builder::aptos_framework_path;
			use aptos_release_builder::components::feature_flags::Features;
			use aptos_sdk::move_types::gas_algebra::GasQuantity;
			use maptos_framework_release_util::{Release, ReleaseBundleError};

			pub struct $struct_name {
				with_features: SetFeatureFlags<super::$struct_name>,
			}

			impl $struct_name {
				pub fn new() -> Self {
					let features = $features_stanza;

					Self {
						with_features: SetFeatureFlags::new(
							super::$struct_name::new(),
							"null",
							"null",
							6,
							Some(aptos_framework_path()), // just use the path to the framework for the gas upgrade
							features,
						),
					}
				}
			}

			impl Release for $struct_name {
				fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
					self.with_features.release_bundle()
				}

				async fn release(
					&self,
					signer: &impl maptos_framework_release_util::ReleaseSigner,
					max_gas_amount: u64,
					gas_unit_price: u64,
					expiration_timestamp_secs: u64,
					client: &aptos_sdk::rest_client::Client,
				) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
					self.with_features
						.release(
							signer,
							max_gas_amount,
							gas_unit_price,
							expiration_timestamp_secs,
							client,
						)
						.await
				}
			}
		}
	};
}
