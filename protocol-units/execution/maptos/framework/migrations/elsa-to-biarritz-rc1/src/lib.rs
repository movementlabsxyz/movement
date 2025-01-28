pub mod dot_movement;

use maptos_framework_release_util::ReleaseSigner;
use std::future::Future;

pub struct ElsaToBiarritzRc1;

impl ElsaToBiarritzRc1 {
	pub fn new() -> Self {
		Self
	}

	pub async fn migrate_framework_from_elsa_to_biarritz_rc1(
		&self,
		client: &aptos_sdk::rest_client::Client,
		signer: &impl ReleaseSigner,
	) -> Result<(), ElsaToBiarritzRc1Error> {
		// todo: validate that the current release is Elsa

		// upgrade to Biarritz RC1 with the gas upgrade

		Ok(())
	}
}

/// Errors thrown by ElsaToBiarritzRc1 migrations.
#[derive(Debug, thiserror::Error)]
pub enum ElsaToBiarritzRc1Error {
	#[error("migration failed: {0}")]
	MigrationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait MigrateElsaToBiarritzRc1 {
	/// Migrate from Elsa to Biarritz RC1.
	fn migrate_framework_from_elsa_to_biarritz_rc1(
		&self,
	) -> impl Future<Output = Result<(), ElsaToBiarritzRc1Error>>;
}
