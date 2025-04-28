pub mod compiler;
pub mod voter;

use aptos_framework::{ReleaseBundle, ReleasePackage};
use aptos_release_builder::aptos_framework_path;
// use aptos_release_builder::components::feature_flags::{FeatureFlag, Features};
use aptos_release_builder::components::framework::{
	generate_upgrade_proposals_release_packages_with_repo, FrameworkReleaseConfig,
};
use aptos_sdk::types::account_config::aptos_test_root_address;
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
use aptos_types::on_chain_config::{FeatureFlag as AptosFeatureFlag, Features as AptosFeatures};
use movement::account::key_rotation::lookup_address;
use std::future::Future;
use std::path::PathBuf;
use tracing::info;

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

	/// Associated method for getting the account address of the signer.
	fn default_release_account_address(
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

	/// Gets the account address of the signer.
	fn release_account_address(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<AccountAddress, ReleaseSignerError>> {
		async move { self.default_release_account_address(client).await }
	}

	/// Get the release account sequence number.
	fn release_account_sequence_number(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<u64, ReleaseSignerError>> {
		async move {
			let account_address = self.release_account_address(client).await?;
			let account = client
				.get_account(account_address)
				.await
				.map_err(|e| ReleaseSignerError::AccountAddressNotFound(Box::new(e)))?;
			Ok(account.into_inner().sequence_number)
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

	fn release_account_address(
		&self,
		client: &Client,
	) -> impl Future<Output = Result<AccountAddress, ReleaseSignerError>> {
		async move {
			// if the override is set, return the override
			if let Some(account_address) = self.account_address {
				return Ok(account_address);
			}

			// otherwise use the default implementation
			self.default_release_account_address(client).await
		}
	}
}

/// A [ReleaseSigner] that signs the transactions with an account address override.
pub struct OverrideAccountAddressReleaseSigner<R>
where
	R: ReleaseSigner,
{
	/// The account address to use for signing.
	pub account_address: AccountAddress,
	/// The underlying release signer.
	pub release_signer: R,
}

impl<R> OverrideAccountAddressReleaseSigner<R>
where
	R: ReleaseSigner,
{
	pub fn new(account_address: AccountAddress, release_signer: R) -> Self {
		Self { account_address, release_signer }
	}

	pub fn core_resource_account(release_signer: R) -> Self {
		Self::new(aptos_test_root_address(), release_signer)
	}
}

impl<R> ReleaseSigner for OverrideAccountAddressReleaseSigner<R>
where
	R: ReleaseSigner,
{
	fn sign_release(
		&self,
		raw_transaction: RawTransaction,
	) -> impl Future<Output = Result<SignedTransaction, ReleaseSignerError>> {
		self.release_signer.sign_release(raw_transaction)
	}

	fn release_account_authentication_key(
		&self,
	) -> impl Future<Output = Result<AuthenticationKey, ReleaseSignerError>> {
		self.release_signer.release_account_authentication_key()
	}

	fn release_account_address(
		&self,
		_client: &Client,
	) -> impl Future<Output = Result<AccountAddress, ReleaseSignerError>> {
		async move { Ok(self.account_address) }
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

	/// Returns the [Features] for the release.
	fn features(&self) -> Result<AptosFeatures, ReleaseBundleError> {
		Ok(AptosFeatures::default())
	}

	/// Returns the [RawTransaction]s for proposing the release.
	fn proposal_raw_transactions(
		&self,
		account_address: AccountAddress,
		start_sequence_number: u64,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		chain_id: ChainId,
	) -> Result<Vec<RawTransaction>, ReleaseBundleError> {
		let release_bundle = self.release_bundle()?;
		build_release_bundles_raw_transactions(
			&release_bundle,
			account_address,
			start_sequence_number,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp_sec_offset,
			chain_id,
		)
	}

	/// Returns the [SignedTransaction]s for proposing the release.
	fn proposal_signed_transactions(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		client: &Client,
	) -> impl Future<Output = Result<Vec<SignedTransaction>, ReleaseBundleError>> {
		async move {
			// get the account address
			let account_address = signer.release_account_address(client).await?;

			// get the start sequence number
			let start_sequence_number = signer.release_account_sequence_number(client).await?;

			// get the chain id
			let ledger_information = client
				.get_ledger_information()
				.await
				.map_err(|e| ReleaseBundleError::Proposing(Box::new(e)))?;
			let chain_id = ChainId::new(ledger_information.into_inner().chain_id);

			// form the raw transactions
			let raw_transactions = self.proposal_raw_transactions(
				account_address,
				start_sequence_number,
				max_gas_amount,
				gas_unit_price,
				expiration_timestamp_sec_offset,
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
	}

	/// Returns the [Transaction]s for proposing the release.
	fn proposal_transactions(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		client: &Client,
	) -> impl Future<Output = Result<Vec<Transaction>, ReleaseBundleError>> {
		async move {
			let signed_transactions = self
				.proposal_signed_transactions(
					signer,
					max_gas_amount,
					gas_unit_price,
					expiration_timestamp_sec_offset,
					client,
				)
				.await?;
			Ok(signed_transactions
				.into_iter()
				.map(|signed_transaction| Transaction::UserTransaction(signed_transaction))
				.collect())
		}
	}

	/// Submits the release proposals to the network and waits for the transactions to be executed.
	/// Returns the transaction hashes of the submitted proposals.
	fn propose_release(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		client: &Client,
	) -> impl Future<Output = Result<Vec<SignedTransaction>, ReleaseBundleError>> {
		async move {
			// get the signed transactions
			let signed_transactions = self
				.proposal_signed_transactions(
					signer,
					max_gas_amount,
					gas_unit_price,
					expiration_timestamp_sec_offset,
					client,
				)
				.await?;

			// submit and wait for transactions to be executed
			for signed_transaction in &signed_transactions {
				match client.submit_and_wait_bcs(signed_transaction).await.map_err(|e| {
					ReleaseBundleError::Proposing(
						format!("submitting transaction failed with: {:?}", e).into(),
					)
				}) {
					Ok(_) => {}
					Err(e) => {
						info!("release proposal transaction failed, but we will still attempt to vote through: {:?}", e);
					}
				}
			}

			Ok(signed_transactions)
		}
	}

	/// Generates the vote proposals for the release.
	fn vote(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		client: &Client,
	) -> impl Future<Output = Result<Vec<SignedTransaction>, ReleaseBundleError>> {
		async move {
			let voter = voter::Voter::head();
			let signed_transactions = voter
				.vote_consensus(
					signer,
					max_gas_amount,
					gas_unit_price,
					expiration_timestamp_sec_offset,
					client,
				)
				.await?;
			Ok(signed_transactions)
		}
	}

	/// Proposes and votes through the release proposals.
	/// This only works when the signer represents a controlling interest in the network.
	fn release(
		&self,
		signer: &impl ReleaseSigner,
		max_gas_amount: u64,
		gas_unit_price: u64,
		expiration_timestamp_sec_offset: u64,
		client: &Client,
	) -> impl Future<Output = Result<Vec<SignedTransaction>, ReleaseBundleError>> {
		async move {
			info!("Proposing release");
			// propose the release
			let completed_proposals = self
				.propose_release(
					signer,
					max_gas_amount,
					gas_unit_price,
					expiration_timestamp_sec_offset,
					client,
				)
				.await?;

			// vote through the proposals
			let now_u64 = std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.map_err(|e| ReleaseBundleError::Build(e.into()))?
				.as_micros() as u64;
			let expiration_timestamp = now_u64 + expiration_timestamp_sec_offset as u64;
			let _completed_votes = self
				.vote(signer, max_gas_amount, gas_unit_price, expiration_timestamp, client)
				.await?;
			info!("Voted through release");

			Ok(completed_proposals)
		}
	}
}

fn build_release_bundles_raw_transactions(
	release_bundle: &ReleaseBundle,
	account_address: AccountAddress,
	start_sequence_number: u64,
	max_gas_amount: u64,
	gas_unit_price: u64,
	expiration_timestamp_sec_offset: u64,
	chain_id: ChainId,
) -> Result<Vec<RawTransaction>, ReleaseBundleError> {
	let payloads = build_release_bundle_transaction_payloads(release_bundle)?;
	let mut transactions = vec![];
	let mut sequence_number = start_sequence_number;
	let mut i = 0;
	for payload in payloads {
		let now_u64 = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map_err(|e| ReleaseBundleError::Build(e.into()))?
			.as_micros() as u64;
		let expiration_timestamp = now_u64 + (expiration_timestamp_sec_offset * i) as u64;

		let raw_transaction = RawTransaction::new(
			account_address,
			sequence_number,
			payload,
			max_gas_amount,
			gas_unit_price,
			expiration_timestamp,
			chain_id,
		);
		transactions.push(raw_transaction);
		sequence_number += 1;
		i += 1;
	}
	Ok(transactions)
}

fn build_release_bundle_transaction_payloads(
	release_bundle: &ReleaseBundle,
) -> Result<Vec<TransactionPayload>, ReleaseBundleError> {
	let mut built_packages = vec![];
	let mut i = 0;
	for release_package in &release_bundle.packages {
		let args = vec![];
		// make the name the lower underscore case of the release package name and the index
		let name = format!("{}_{}", i, release_package.name().to_lowercase());
		let payload = build_release_package_transaction_payload(&name, args, release_package)?;
		built_packages.push(payload);
		i += 1;
	}
	Ok(built_packages)
}

fn build_release_package_transaction_payload(
	name: &str,
	args: Vec<TransactionArgument>,
	release_package: &ReleasePackage,
) -> Result<TransactionPayload, ReleaseBundleError> {
	println!("Modules in release package {}: ", name);
	for module in &release_package.package_metadata().modules {
		println!("module: {:?}", module.name);
	}

	let script_path = PathBuf::from(".debug/move-scripts").join(name).with_extension("move");
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

	let compiler = compiler::Compiler::movement();
	let code =
		compiler
			.compile_in_temp_dir_to_bytecode(name, script_path.as_path())
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
#[derive(Debug)]
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

		println!("Generating upgrade proposals for {:?} in {}", config, repo);
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
		const $bytes_name: &[u8] = include_bytes!(concat!("..", "/", "mrb_cache", "/", $path));

		#[cfg(windows)]
		const $bytes_name: &[u8] = include_bytes!(concat!("..", "\\", "mrb_cache", "\\", $path));

		// Define the struct implementing Release
		#[derive(Debug)]
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
		#[derive(Debug)]
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
			// Write to mrb_cache/<mrb_file>-<commit_hash>
			let target_cache_dir = PathBuf::from("mrb_cache");
			std::fs::create_dir_all(&target_cache_dir)
				.context("failed to create cache directory")?;
			let path = target_cache_dir.join(format!("{}-{}", COMMIT_HASH, MRB_FILE));

			// rerun if the file on the path has for some reason changed
			println!("cargo:rerun-if-changed={}", path.to_str().unwrap());

			// if the release is already built and CACHE_RELEASE is set, skip building
			let force_build_all_releases = std::env::var(FORCE_BUILD_ALL_RELEASES).is_ok();
			let force_build_release = std::env::var(FORCE_BUILD_RELEASE).is_ok();
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
