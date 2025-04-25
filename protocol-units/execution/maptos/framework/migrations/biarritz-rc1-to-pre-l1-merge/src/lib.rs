pub mod dot_movement;
pub use dot_movement::*;

use aptos_framework_biarritz_rc1_release::cached::full::feature_upgrade::BiarritzRc1;
use maptos_framework_release_util::{Release, ReleaseSigner};
use std::future::Future;

pub struct BiarritzRc1ToPreL1Merge;

impl BiarritzRc1ToPreL1Merge {
	pub fn new() -> Self {
		Self
	}

	pub async fn migrate_framework_from_biarritz_rc1_to_pre_l1_merge(
		&self,
		client: &aptos_sdk::rest_client::Client,
		framework_signer: &impl ReleaseSigner,
		faucet_signer: &impl ReleaseSigner,
	) -> Result<(), BiarritzRc1ToPreL1MergeError> {
		// First perform framework upgrade with core resource account override
		let biarritz_rc1 = BiarritzRc1::new();
		biarritz_rc1
			.release(framework_signer, 2_000_000, 100, 60_000, client)
			.await
			.map_err(|e| BiarritzRc1ToPreL1MergeError::MigrationFailed(e.into()))?;

		// Then perform faucet operations with regular signer
		// TODO: Add faucet operations here using faucet_signer

		Ok(())
	}
}

/// Errors thrown by BiarritzRc1ToPreL1Merge migrations.
#[derive(Debug, thiserror::Error)]
pub enum BiarritzRc1ToPreL1MergeError {
	#[error("migration failed: {0}")]
	MigrationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait MigrateBiarritzRc1ToPreL1Merge {
	/// Migrate from Elsa to Biarritz RC1.
	fn migrate_framework_from_biarritz_rc1_to_pre_l1_merge(
		&self,
	) -> impl Future<Output = Result<(), BiarritzRc1ToPreL1MergeError>>;
}
