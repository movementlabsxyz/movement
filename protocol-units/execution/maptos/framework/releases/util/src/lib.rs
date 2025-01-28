pub mod compiler;

use aptos_framework::{ReleaseBundle, ReleasePackage};
use aptos_release_builder::aptos_framework_path;
use aptos_release_builder::components::framework::{
	generate_upgrade_proposals_release_packages_with_repo, FrameworkReleaseConfig,
};
use aptos_sdk::{
	rest_client::Client,
	types::{
		account_address::AccountAddress,
		chain_id::ChainId,
		transaction::authenticator::AuthenticationKey,
		transaction::{
			RawTransaction, Script, SignedTransaction, Transaction, TransactionArgument,
			TransactionPayload,
		},
		LocalAccount,
	},
};
use movement::account::key_rotation::lookup_address;
use std::future::Future;
use std::path::PathBuf;

use std::error;

#[derive(Debug, thiserror::Error)]
pub enum ReleaseSignerError {
	#[error("signing release failed with: {0}")]
	Signing(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("account address for release not found: {0}")]
	AccountAddressNotFound(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait ReleaseSigner {
	/// Signs the given raw transaction.
	fn sign_release(
		&self,
		raw_transaction: RawTransaction,
	) -> impl Future<Output = Result<SignedTransaction, ReleaseSignerError>>;

	/// Gets the authentication key of the signer.
	fn release_account_authentication_key(
		&self,
	) -> impl Future<Output = Result<AuthenticationKey, ReleaseSignerError>>;

	/// Gets the account address of the signer.
	fn release_account_address(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<AccountAddress, ReleaseSignerError>> {
		async move {
			// get the authentication key
			let authentication_key = self.release_account_authentication_key().await?;

			// form the lookup address from the authentication key
			let lookup = AccountAddress::new(*authentication_key.account_address());

			// lookup the account address
			let account_address = lookup_address(client, lookup, true)
				.await
				.map_err(|e| ReleaseSignerError::AccountAddressNotFound(Box::new(e)))?;

			Ok(account_address)
		}
	}
}

/// A [ReleaseSigner] that signs the transactions with a local account.
pub struct LocalAccountReleaseSigner {
	/// The local account to sign the transactions with.
	pub local_account: LocalAccount,
	/// An override for the account address.
	pub account_address: Option<AccountAddress>,
}

impl LocalAccountReleaseSigner {
	pub fn new(local_account: LocalAccount, account_address: Option<AccountAddress>) -> Self {
		Self { local_account, account_address }
	}
}

impl ReleaseSigner for LocalAccountReleaseSigner {
	fn sign_release(
		&self,
		raw_transaction: RawTransaction,
	) -> impl Future<Output = Result<SignedTransaction, ReleaseSignerError>> {
		let signed_transaction_res = self.local_account.sign_transaction(raw_transaction);
		async move { Ok(signed_transaction_res) }
	}

	async fn release_account_authentication_key(
		&self,
	) -> Result<AuthenticationKey, ReleaseSignerError> {
		Ok(self.local_account.authentication_key())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseBundleError {
	#[error("building release failed with error: {0}")]
	Build(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("signing release bundle failed with error: {0}")]
	Signing(#[from] ReleaseSignerError),
	#[error("proposing release failed with error: {0}")]
	Proposing(#[source] Box<dyn error::Error + Send + Sync>),
}

pub trait Release {
	/// Returns a [ReleaseBundle] that contains the release packages.
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError>;

	/// Returns the [RawTransaction]s for proposing the release.
	fn proposal_raw_transactions(
		&self,
		account_address: AccountAddress,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: ChainId,
	) -> Result<Vec<RawTransaction>, ReleaseBundleError> {
		let release_bundle = self.release_bundle()?;
		build_release_bundles_raw_transactions(
			&release_bundle,
			account_address,
			start_sequence_number,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			chain_id,
		)
	}

	/// Returns the [SignedTransaction]s for proposing the release.
	async fn proposal_signed_transactions(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: ChainId,
		client: &Client,
	) -> Result<Vec<SignedTransaction>, ReleaseBundleError> {
		// get the account address
		let account_address = signer.release_account_address(client).await?;

		// form the raw transactions
		let raw_transactions = self.proposal_raw_transactions(
			account_address,
			start_sequence_number,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			chain_id,
		)?;

		// sign the raw transactions
		let mut signed_transactions = vec![];
		for raw_transaction in raw_transactions {
			let signed_transaction = signer.sign_release(raw_transaction).await?;
			signed_transactions.push(signed_transaction);
		}

		Ok(signed_transactions)
	}

	/// Returns the [Transaction]s for proposing the release.
	async fn proposal_transactions(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: ChainId,
		client: &Client,
	) -> Result<Vec<Transaction>, ReleaseBundleError> {
		let signed_transactions = self
			.proposal_signed_transactions(
				signer,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				chain_id,
				client,
			)
			.await?;
		Ok(signed_transactions
			.into_iter()
			.map(|signed_transaction| Transaction::UserTransaction(signed_transaction))
			.collect())
	}

	/// Submits the release proposals to the network.
	/// Returns the transaction hashes of the submitted proposals.
	async fn submit_release_proposals(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: ChainId,
		client: &Client,
	) -> Result<Vec<SignedTransaction>, ReleaseBundleError> {
		// form the signed transactions
		let transactions = self
			.proposal_signed_transactions(
				signer,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				chain_id,
				client,
			)
			.await?;

		// submit the transactions
		let transaction_batch_submission_res = client
			.submit_batch_bcs(&transactions)
			.await
			.map_err(|e| ReleaseBundleError::Proposing(Box::new(e)))?;

		let transaction_failures =
			transaction_batch_submission_res.into_inner().transaction_failures;

		if !transaction_failures.is_empty() {
			return Err(ReleaseBundleError::Proposing(
				format!("transaction failures: {:?}", transaction_failures).into(),
			));
		}

		Ok(transactions)
	}

	/// Submits the release proposals to the network and waits for the transactions to be executed.
	/// Returns the transaction hashes of the submitted proposals.
	async fn propose_release(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: ChainId,
		client: &Client,
	) -> Result<Vec<SignedTransaction>, ReleaseBundleError> {
		let submitted_transactions = self
			.submit_release_proposals(
				signer,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				chain_id,
				client,
			)
			.await?;

		// wait for the transactions to be executed
		for transaction in &submitted_transactions {
			client.wait_for_signed_transaction_bcs(transaction).await.map_err(|e| {
				ReleaseBundleError::Proposing(
					format!(
						"waiting for transaction {:?} failed with: {:?}",
						transaction.committed_hash(),
						e
					)
					.into(),
				)
			})?;
		}

		Ok(submitted_transactions)
	}

	/// Proposes and votes through the release proposals.
	/// This only works when the signer represents a controlling interest in the network.
	async fn release(
		&self,
		signer: &impl ReleaseSigner,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_secs: u64,
		chain_id: ChainId,
		client: &Client,
	) -> Result<Vec<SignedTransaction>, ReleaseBundleError> {
		// propose the release
		let completed_proposals = self
			.propose_release(
				signer,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_secs,
				chain_id,
				client,
			)
			.await?;

		// vote through the proposals
		// todo: currently we are not voting through the proposals, the scripts will simply execute under the root signer
		// write out the bytecode for t

		Ok(completed_proposals)
	}
}

fn build_release_bundles_raw_transactions(
	release_bundle: &ReleaseBundle,
	account_address: AccountAddress,
	start_sequence_number: u64,
	max_gas_amount: u64,
	gas_unit_price: u64,
	expiration_timestamp_secs: u64,
	chain_id: ChainId,
) -> Result<Vec<RawTransaction>, ReleaseBundleError> {
	let payloads = build_release_bundle_transaction_payloads(release_bundle)?;
	let mut transactions = vec![];
	let mut sequence_number = start_sequence_number;
	for payload in payloads {
		let raw_transaction = RawTransaction::new(
			account_address,
			sequence_number,
			payload,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_secs,
			chain_id,
		);
		transactions.push(raw_transaction);
		sequence_number += 1;
	}
	Ok(transactions)
}

fn build_release_bundle_transaction_payloads(
	release_bundle: &ReleaseBundle,
) -> Result<Vec<TransactionPayload>, ReleaseBundleError> {
	let mut built_packages = vec![];
	for release_package in &release_bundle.packages {
		let args = vec![];
		let payload = build_release_package_transaction_payload(args, release_package)?;
		built_packages.push(payload);
	}
	Ok(built_packages)
}

fn build_release_package_transaction_payload(
	args: Vec<TransactionArgument>,
	release_package: &ReleasePackage,
) -> Result<TransactionPayload, ReleaseBundleError> {
	println!("release_package_code: {:?}", release_package.code().len());

	// use .debug/move-scripts/{package.package_metadata.name()} to write out the script
	let script_path = PathBuf::from(".debug/move-scripts/")
		.join(release_package.package_metadata().name.clone())
		.with_extension("move");
	// create all parent directories
	std::fs::create_dir_all(script_path.parent().unwrap()).map_err(|e| {
		ReleaseBundleError::Build(
			format!("failed to create parent directories for script path: {:?}", e).into(),
		)
	})?;

	release_package
		.generate_script_proposal_testnet(AccountAddress::ONE, script_path.clone())
		.map_err(|e| {
			ReleaseBundleError::Build(
				format!("failed to generate script proposal for release package: {:?}", e).into(),
			)
		})?;

	let compiler = compiler::Compiler::new(
		"doesn't matter",
		"doesn't matter",
		6,
		Some(aptos_framework_path()),
	);
	let code = compiler
		.compile_in_temp_dir_to_bytecode("proposal", script_path.as_path())
		.map_err(|e| {
			ReleaseBundleError::Build(
				format!("failed to compile script in temp dir to bytecode: {:?}", e).into(),
			)
		})?;

	let script_payload = TransactionPayload::Script(Script::new(code, vec![], args));

	Ok(script_payload)
}

/// To form a commit hash proposer, at the lowest level we use [generate_upgrade_proposals_with_repo] function to generate the scripts.
/// We then write these scripts out to a proposal directory in line with the implementation here: https://github.com/movementlabsxyz/aptos-core/blob/ac9de113a4afec6a26fe587bb92c982532f09d3a/aptos-move/aptos-release-builder/src/components/mod.rs#L563
/// We then need to compile the code to form [ReleasePackage]s which are then used to form [ReleaseBundle]s.
/// To do this, we need to form a [BuiltPackage] from the scripts I BELIEVE.
pub struct CommitHash {
	pub repo: &'static str,
	pub commit_hash: &'static str,
	pub bytecode_version: &'static u32,
}

impl CommitHash {
	pub fn new(
		repo: &'static str,
		commit_hash: &'static str,
		bytecode_version: &'static u32,
	) -> Self {
		Self { repo, commit_hash, bytecode_version }
	}

	pub fn framework_release_config(&self) -> (FrameworkReleaseConfig, &'static str) {
		let config = FrameworkReleaseConfig {
			bytecode_version: *self.bytecode_version,
			git_hash: Some(self.commit_hash.to_string()),
		};
		(config, self.repo)
	}
}

impl Release for CommitHash {
	fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
		let (config, repo) = self.framework_release_config();

		let (_commit_info, releases) =
			generate_upgrade_proposals_release_packages_with_repo(&config, true, vec![], repo)
				.map_err(|e| ReleaseBundleError::Build(e.into()))?;
		let release_packages = releases
			.into_iter()
			.map(|(_account, release_package, _move_script_path, _script_name)| release_package)
			.collect();

		let release_bundle = ReleaseBundle::new(release_packages, vec![]);

		Ok(release_bundle)
	}
}

#[macro_export]
macro_rules! mrb_release {
	($struct_name:ident, $bytes_name:ident, $path:expr) => {
		use aptos_framework::ReleaseBundle;
		use maptos_framework_release_util::{Release, ReleaseBundleError};

		// Define the constant with the byte data
		#[cfg(unix)]
		const $bytes_name: &[u8] =
			include_bytes!(concat!("..", "/", "target", "/", "mrb_cache", "/", $path));

		#[cfg(windows)]
		const $bytes_name: &[u8] =
			include_bytes!(concat!("..", "\\", "target", "\\", "mrb_cache", "\\", $path));

		// Define the struct implementing Release
		pub struct $struct_name;

		impl $struct_name {
			pub fn new() -> Self {
				Self
			}
		}

		impl Release for $struct_name {
			fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
				let release_bundle: ReleaseBundle = bcs::from_bytes($bytes_name)
					.map_err(|e| ReleaseBundleError::Build(e.into()))?;
				Ok(release_bundle)
			}
		}
	};
}

#[macro_export]
macro_rules! commit_hash_with_script {
	($name:ident, $repo:expr, $commit_hash:expr, $bytecode_version:expr, $mrb_file:expr, $cache_env_var:expr) => {
		use anyhow::Context;
		use std::path::PathBuf;

		use aptos_framework::ReleaseBundle;
		use maptos_framework_release_util::{CommitHash, Release, ReleaseBundleError};

		pub static REPO: &str = $repo;
		pub static COMMIT_HASH: &str = $commit_hash;
		pub static BYTECODE_VERSION: u32 = $bytecode_version;
		pub static FORCE_BUILD_RELEASE: &str = $cache_env_var;
		pub static FORCE_BUILD_ALL_RELEASES: &str = "FORCE_BUILD_ALL_FRAMEWORK_RELEASES";
		pub static MRB_FILE: &str = $mrb_file;

		/// Builds a release for the specified framework.
		/// This is a wrapper around the [CommitHash] builder.
		pub struct $name(CommitHash);

		impl $name {
			pub fn new() -> Self {
				Self(CommitHash::new(REPO, COMMIT_HASH, &BYTECODE_VERSION))
			}
		}

		impl Release for $name {
			fn release_bundle(&self) -> Result<ReleaseBundle, ReleaseBundleError> {
				self.0.release_bundle()
			}
		}

		pub fn main() -> Result<(), anyhow::Error> {
			// Write to mrb_cache/<mrb_file>
			let target_cache_dir = PathBuf::from("mrb_cache");
			std::fs::create_dir_all(&target_cache_dir)
				.context("failed to create cache directory")?;
			let path = target_cache_dir.join(MRB_FILE);

			// if the release is already built and CACHE_RELEASE is set, skip building
			let force_build_all_releases = std::env::var(FORCE_BUILD_ALL_RELEASES).is_ok();
			let force_build_release = std::env::var(FORCE_BUILD_ALL_RELEASES).is_ok();
			let path_exists = std::fs::metadata(&path).is_ok();

			if (!force_build_release || !force_build_all_releases) && path_exists {
				println!("Release already built, skipping build");
				return Ok(());
			}

			// serialize the release
			let release = $name::new();
			let release_bundle = release.release_bundle()?;
			let serialized_release =
				bcs::to_bytes(&release_bundle).context("failed to serialize release")?;

			std::fs::write(&path, serialized_release).context("failed to write release")?;

			Ok(())
		}
	};
}
