pub mod dot_movement;
pub use dot_movement::*;

use aptos_framework_pre_l1_merge_release::cached::full::feature_upgrade::PreL1Merge;
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
		signer: &impl ReleaseSigner,
	) -> Result<(), BiarritzRc1ToPreL1MergeError> {
		// todo: validate that the current release is Biarritz RC1

		// upgrade to Pre-L1 Merge with the gas upgrade
		let pre_l1_merge = PreL1Merge::new();
		pre_l1_merge
			.release(signer, 2_000_000, 100, 60_000, client)
			.await
			.map_err(|e| BiarritzRc1ToPreL1MergeError::MigrationFailed(e.into()))?;

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
	/// Migrate from Biarritz RC1 to Pre-L1 Merge.
	fn migrate_framework_from_biarritz_rc1_to_pre_l1_merge(
		&self,
	) -> impl Future<Output = Result<(), BiarritzRc1ToPreL1MergeError>>;
}
