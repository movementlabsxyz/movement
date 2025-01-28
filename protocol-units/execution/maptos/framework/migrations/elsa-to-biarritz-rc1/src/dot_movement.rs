use crate::{ElsaToBiarritzRc1, ElsaToBiarritzRc1Error, MigrateElsaToBiarritzRc1};
use dot_movement::DotMovement;
use maptos_framework_release_util::ReleaseSigner;
use movement_config::{ops::aptos::rest_client::RestClient, Config};

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

		Ok(())
	}
}
