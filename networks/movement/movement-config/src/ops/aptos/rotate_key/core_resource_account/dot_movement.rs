use crate::ops::aptos::rest_client::RestClientOperations;
use crate::ops::aptos::rotate_key::core_resource_account::{
	load_key_rotation_signer::LoadKeyRotationSigner, signer::CoreResourceAccountKeyRotationSigner,
	RotateCoreResourceAccountError, RotateCoreResourceAccountKey,
	RotateCoreResourceAccountKeyOperations,
};
use crate::releases::biarritz_rc1::Config;
use dot_movement::DotMovement;

impl RotateCoreResourceAccountKeyOperations for DotMovement {
	async fn rotate_core_resource_account_key(
		&self,
		new_signer: &impl CoreResourceAccountKeyRotationSigner,
	) -> Result<Config, RotateCoreResourceAccountError> {
		// get the config value
		let config: Config = self.try_get_config_from_json().map_err(|e| {
			RotateCoreResourceAccountError::KeyRotationFailed(
				format!("failed to get config from json: {}", e).into(),
			)
		})?;

		// load the old signer
		let old_signer = config.load_key_rotation_signer().await.map_err(|e| {
			RotateCoreResourceAccountError::KeyRotationFailed(
				format!("failed to load key rotation signer: {}", e).into(),
			)
		})?;

		// load the rest client
		let client = config.get_rest_client().await.map_err(|e| {
			RotateCoreResourceAccountError::KeyRotationFailed(
				format!("failed to get rest client: {}", e).into(),
			)
		})?;

		// use the rotator helper to get the new config
		let rotator = RotateCoreResourceAccountKey::new();
		let updated_config = rotator
			.rotate_core_resource_account_key(config, &client, &old_signer, new_signer)
			.await?;

		// write the migrated value
		self.try_overwrite_config_to_json(&updated_config)
			.map_err(|e| RotateCoreResourceAccountError::KeyRotationFailed(e.into()))?;

		Ok(updated_config)
	}
}
