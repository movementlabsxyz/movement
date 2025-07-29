use crate::common_args::MovementArgs;
use clap::Parser;
use movement_config::ops::aptos::rotate_key::core_resource_account::{
	load_key_rotation_signer::Signer, RotateCoreResourceAccountKeyOperations,
};
use movement_signer::key::TryFromCanonicalString;
use movement_signer_loader::identifiers::SignerIdentifier;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Rotates the key for a core resource account.")]
pub struct CoreResourceAccount {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
	pub new_signer_identifier: String,
}

impl CoreResourceAccount {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		// get the movement config from dot movement
		let dot_movement = self.movement_args.dot_movement()?;

		// load the new signer from the canonical identifier
		let identifier = SignerIdentifier::try_from_canonical_string(&self.new_signer_identifier)
			.map_err(|e| anyhow::anyhow!(e))?;
		let new_signer = Signer::load_from_identifier(identifier).await?;

		// run the core resource account key rotation
		dot_movement.rotate_core_resource_account_key(&new_signer).await?;

		Ok(())
	}
}
