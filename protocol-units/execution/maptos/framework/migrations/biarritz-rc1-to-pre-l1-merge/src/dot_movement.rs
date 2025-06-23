use crate::{
	BiarritzRc1ToPreL1Merge, BiarritzRc1ToPreL1MergeError, MigrateBiarritzRc1ToPreL1Merge,
};
use dot_movement::DotMovement;
use maptos_framework_release_util::OverrideAccountAddressReleaseSigner;
use movement_config::{
	ops::aptos::{
		framework::releases::release_signer::ReleaseSignerOperations,
		rest_client::RestClientOperations,
	},
	Config,
};

impl MigrateBiarritzRc1ToPreL1Merge for DotMovement {
	async fn migrate_framework_from_biarritz_rc1_to_pre_l1_merge(
		&self,
	) -> Result<(), BiarritzRc1ToPreL1MergeError> {
		// get the movement config from dot movement
		let config = self.try_get_config_from_json::<Config>().map_err(|e| {
			BiarritzRc1ToPreL1MergeError::MigrationFailed(
				format!("failed to get config: {}", e).into(),
			)
		})?;

		// get the rest client from the movement config
		let rest_client = config
			.get_rest_client()
			.await
			.map_err(|e| BiarritzRc1ToPreL1MergeError::MigrationFailed(e.into()))?;

		// get the release signer from the movement config
		let signer = config
			.get_release_signer()
			.await
			.map_err(|e| BiarritzRc1ToPreL1MergeError::MigrationFailed(e.into()))?;

		// write the signer with a core resource account override
		let signer = OverrideAccountAddressReleaseSigner::core_resource_account(signer);

		// migrate the framework from Biarritz RC1 to Pre-L1 Merge
		let biarritz_rc1_to_pre_l1_merge = BiarritzRc1ToPreL1Merge::new();
		biarritz_rc1_to_pre_l1_merge
			.migrate_framework_from_biarritz_rc1_to_pre_l1_merge(&rest_client, &signer)
			.await?;

		Ok(())
	}
}
