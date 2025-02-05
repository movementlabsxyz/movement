use crate::{ElsaToBiarritzRc1, ElsaToBiarritzRc1Error, MigrateElsaToBiarritzRc1};
use dot_movement::DotMovement;
use maptos_framework_release_util::OverrideAccountAddressReleaseSigner;
use movement_config::{
	ops::aptos::{
		framework::releases::release_signer::ReleaseSignerOperations,
		rest_client::RestClientOperations,
	},
	Config,
};

impl MigrateElsaToBiarritzRc1 for DotMovement {
	async fn migrate_framework_from_elsa_to_biarritz_rc1(
		&self,
	) -> Result<(), ElsaToBiarritzRc1Error> {
		// get the movement config from dot movement
		let config = self.try_get_config_from_json::<Config>().map_err(|e| {
			ElsaToBiarritzRc1Error::MigrationFailed(format!("failed to get config: {}", e).into())
		})?;

		// get the rest client from the movement config
		let rest_client = config
			.get_rest_client()
			.await
			.map_err(|e| ElsaToBiarritzRc1Error::MigrationFailed(e.into()))?;

		// get the release signer from the movement config
		let signer = config
			.get_release_signer()
			.await
			.map_err(|e| ElsaToBiarritzRc1Error::MigrationFailed(e.into()))?;

		// write the signer with a core resource account override
		let signer = OverrideAccountAddressReleaseSigner::core_resource_account(signer);

		// migrate the framework from Elsa to Biarritz RC1
		let elsa_to_biarritz_rc1 = ElsaToBiarritzRc1::new();
		elsa_to_biarritz_rc1
			.migrate_framework_from_elsa_to_biarritz_rc1(&rest_client, &signer)
			.await?;

		Ok(())
	}
}
