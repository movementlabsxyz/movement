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
		config: &MovementConfig,
	) -> Result<(), BiarritzRc1ToPreL1MergeError> {
		let rest_client = config.rest_client().await?;
		let signer = config.release_signer().await?;

		// Use core resource account override for framework operations
		let framework_signer =
			OverrideAccountAddressReleaseSigner::core_resource_account(signer.clone());

		// Use regular signer for faucet operations
		let faucet_signer = signer;

		BiarritzRc1ToPreL1Merge::new()
			.migrate_framework_from_biarritz_rc1_to_pre_l1_merge(
				&rest_client,
				&framework_signer,
				&faucet_signer,
			)
			.await
	}
}
