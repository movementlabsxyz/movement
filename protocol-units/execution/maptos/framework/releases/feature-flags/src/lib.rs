use aptos_framework::ReleaseBundle;
use aptos_release_builder::components::feature_flags::{
	generate_feature_upgrade_proposal, Features,
};
use aptos_sdk::types::transaction::{Script, SignedTransaction, TransactionPayload};
use core::fmt::Debug;
use maptos_framework_release_util::{
	compiler::Compiler, Release, ReleaseBundleError, ReleaseSigner,
};
use std::fs;
use tempfile::tempdir;
use tracing::info;

/// [SetFeatureFlags] can be used to wrap a proposal to prefix it with a feature flag.
#[derive(Debug)]
pub struct SetFeatureFlags<R>
where
	R: Release + Debug,
{
	pub wrapped_release: R,
	pub features: Features,
}

impl<R> SetFeatureFlags<R>
where
	R: Release + Debug,
{
	pub fn new(wrapped_release: R, features: Features) -> Self {
		Self { wrapped_release, features }
	}

	/// Generates the bytecode for the feature flag proposal.
	pub fn set_feature_flags_proposal_bytecode(&self) -> Result<Vec<u8>, ReleaseBundleError> {
		let (_, update_feature_flags_script) =
			generate_feature_upgrade_proposal(&self.features, true, vec![])
				.map_err(|e| ReleaseBundleError::Build(e.into()))?
				.pop()
				.map_or(Err(ReleaseBundleError::Build("no feature flag proposal".into())), Ok)?;

		let temp_dir = tempdir().map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let feature_flags_script_path = temp_dir.path().join("feature_flags");
		let mut feature_flags_script_path = feature_flags_script_path.as_path().to_path_buf();
		feature_flags_script_path.set_extension("move");
		fs::write(feature_flags_script_path.as_path(), update_feature_flags_script)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		// list all files in the temp dir
		let files =
			fs::read_dir(temp_dir.path()).map_err(|e| ReleaseBundleError::Build(e.into()))?;
		for file in files {
			let file = file.map_err(|e| ReleaseBundleError::Build(e.into()))?;
			println!("file: {:?}", file.path());
		}

		let compiler = crate::get_compiler_from_env();
		let bytecode = compiler
			.compile_in_temp_dir_to_bytecode("feature_flags", &feature_flags_script_path)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		Ok(bytecode)
	}

	/// Generate the transaction for the feature flag proposal.
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
		info!("Setting feature flags");
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
			info!("failed to submit feature flag proposal: {:?}", e);
			ReleaseBundleError::Proposing(
				format!("failed to submit feature flag proposal: {:?}", e).into(),
			)
		})?;

		info!("Feature flags set");

		Ok(vec![signed_transaction])
	}
}

impl<R> Release for SetFeatureFlags<R>
where
	R: Release + Debug,
{
	/// Note: the release bundle will not actually contain the feature flag proposal, so when running genesis with this release, the feature flag proposal will not be included.
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
		info!("Proposing release before feature flags {:?}", self);
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

		// generate and execute the feature flag proposal
		info!("Setting feature flags");
		let now_u64 = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?
			.as_micros() as u64;
		let expiration_timestamp_secs = now_u64 + expiration_timestamp_sec_offset;
		self.set_feature_flags(
			signer,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			client,
		)
		.await?;
		info!("Feature flags set");

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
			use aptos_types::on_chain_config::Features as AptosFeatures;
			use maptos_framework_release_util::{Release, ReleaseBundleError};
			use tracing::info;

			pub struct $struct_name {
				pub with_features: SetFeatureFlags<super::$struct_name>,
			}

			impl $struct_name {
				pub fn new() -> Self {
					let features = $features_stanza;

					Self {
						with_features: SetFeatureFlags::new(super::$struct_name::new(), features),
					}
				}
			}

			impl Release for $struct_name {
				fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
					self.with_features.release_bundle()
				}

				fn features(&self) -> Result<AptosFeatures, ReleaseBundleError> {
					self.with_features.features().into()
				}

				async fn propose_release(
					&self,
					signer: &impl maptos_framework_release_util::ReleaseSigner,
					max_gas_amount: u64,
					gas_unit_price: u64,
					expiration_timestamp_secs: u64,
					client: &aptos_sdk::rest_client::Client,
				) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError>
				{
					info!("Proposing release {} with feature flags", stringify!($struct_name));
					self.with_features
						.propose_release(
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
