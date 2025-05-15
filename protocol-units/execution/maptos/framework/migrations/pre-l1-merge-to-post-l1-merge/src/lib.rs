pub mod dot_movement;

use aptos_framework_biarritz_rc1_release::cached::full::feature_upgrade::BiarritzRc1;
use maptos_framework_release_util::{Release, ReleaseSigner};
use std::future::Future;

pub struct PreL1MergeToPostL1Merge;

impl PreL1MergeToPostL1Merge {
	pub fn new() -> Self {
		Self
	}

	pub async fn migrate_framework_from_pre_l1_merge_to_post_l1_merge(
		&self,
		client: &aptos_sdk::rest_client::Client,
		signer: &impl ReleaseSigner,
	) -> Result<(), PreL1MergeToPostL1MergeError> {
		// todo: validate that the current release is Elsa

		// upgrade to PostL1Merge with the gas upgrade
		let biarritz_rc1 = BiarritzRc1::new();
		biarritz_rc1
			.release(signer, 2_000_000, 100, 60_000, client)
			.await
			.map_err(|e| PreL1MergeToPostL1MergeError::MigrationFailed(e.into()))?;

		Ok(())
	}
}

/// Errors thrown by PreL1MergeToPostL1Merge migrations.
#[derive(Debug, thiserror::Error)]
pub enum PreL1MergeToPostL1MergeError {
	#[error("migration failed: {0}")]
	MigrationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait MigratePreL1MergeToPostL1Merge {
	/// Migrate from PreL1Merge to PostL1Merge.
	fn migrate_framework_from_biarritza_rc1_to_pre_l1_merge(
		&self,
	) -> impl Future<Output = Result<(), PreL1MergeToPostL1MergeError>>;
}
