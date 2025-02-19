use aptos_framework::ReleaseBundle;
use aptos_sdk::types::transaction::{Script, SignedTransaction, TransactionPayload};
use core::fmt::Debug;
use maptos_framework_release_util::{
	compiler::Compiler, Release, ReleaseBundleError, ReleaseSigner,
};
use std::fs;
use tempfile::tempdir;
use tracing::info;

/// [RunScript] can be used to wrap a proposal to prefix it with a script.
#[derive(Debug)]
pub struct RunScript<R>
where
	R: Release + Debug,
{
	pub wrapped_release: R,
	pub script: String,
}

impl<R> RunScript<R>
where
	R: Release + Debug,
{
	pub fn new(wrapped_release: R, script: String) -> Self {
		Self { wrapped_release, script }
	}

	/// Generates the bytecode for the script proposal.
	pub fn set_release_script_proposal_bytecode(&self) -> Result<Vec<u8>, ReleaseBundleError> {
		let temp_dir = tempdir().map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let release_script_script_path = temp_dir.path().join("release_script");
		let mut release_script_script_path = release_script_script_path.as_path().to_path_buf();
		release_script_script_path.set_extension("move");
		fs::write(release_script_script_path.as_path(), self.script.as_bytes())
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		// list all files in the temp dir
		let files =
			fs::read_dir(temp_dir.path()).map_err(|e| ReleaseBundleError::Build(e.into()))?;
		for file in files {
			let file = file.map_err(|e| ReleaseBundleError::Build(e.into()))?;
			println!("file: {:?}", file.path());
		}

		let compiler = Compiler::movement();

		let bytecode = compiler
			.compile_in_temp_dir_to_bytecode("release_script", &release_script_script_path)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?;

		Ok(bytecode)
	}

	/// Generate the transaction for the script proposal.
	pub async fn set_release_script_proposal_transaction(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<SignedTransaction, ReleaseBundleError> {
		let bytecode = self.set_release_script_proposal_bytecode()?;
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

	pub async fn set_release_script(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		client: &aptos_sdk::rest_client::Client,
	) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
		info!("Setting scripts");
		let signed_transaction = self
			.set_release_script_proposal_transaction(
				signer,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				client,
			)
			.await?;

		let _response = client.submit_and_wait_bcs(&signed_transaction).await.map_err(|e| {
			info!("failed to submit script proposal: {:?}", e);
			ReleaseBundleError::Proposing(
				format!("failed to submit script proposal: {:?}", e).into(),
			)
		})?;

		info!("Release script run");

		Ok(vec![signed_transaction])
	}
}

impl<R> Release for RunScript<R>
where
	R: Release + Debug,
{
	/// Note: the release bundle will not actually contain the script proposal, so when running genesis with this release, the script proposal will not be included.
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
		info!("Proposing release before scripts {:?}", self);
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

		// generate and execute the script proposal
		info!("Setting scripts");
		let now_u64 = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?
			.as_secs();
		let expiration_timestamp_secs = now_u64 + expiration_timestamp_sec_offset;
		self.set_release_script(
			signer,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			client,
		)
		.await?;
		info!("Release script run");

		Ok(transactions)
	}
}

#[macro_export]
// Macro definition
macro_rules! generate_script_module {
	($mod_name:ident, $struct_name:ident, $script_stanza:expr) => {
		pub mod $mod_name {
			use aptos_framework::ReleaseBundle;
			use aptos_framework_release_script_release::RunScript;
			use aptos_release_builder::aptos_framework_path;
			use aptos_sdk::move_types::gas_algebra::GasQuantity;
			use maptos_framework_release_util::{Release, ReleaseBundleError};
			use tracing::info;

			#[derive(Debug)]
			pub struct $struct_name {
				pub with_script: RunScript<super::$struct_name>,
			}

			impl $struct_name {
				pub fn new() -> Self {
					let script = $script_stanza;
					Self { with_script: RunScript::new(super::$struct_name::new(), script) }
				}
			}

			impl Release for $struct_name {
				fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
					self.with_script.release_bundle()
				}

				async fn propose_release(
					&self,
					signer: &impl maptos_framework_release_util::ReleaseSigner,
					max_gas_amount: u64,
					gas_unit_price: u64,
					expiration_timestamp_secs: u64,
					client: &aptos_sdk::rest_client::Client,
				) -> Result<Vec<aptos_types::transaction::SignedTransaction>, ReleaseBundleError> {
					info!("Proposing release {} with scripts", stringify!($struct_name));
					self.with_script
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
