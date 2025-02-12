use crate::migrations::elsa_to_biarritz_rc1::{
	ElsaToBiarritzRc1, ElsaToBiarritzRc1Error, MigrateElsaToBiarritzRc1,
};
use crate::Config;
use dot_movement::DotMovement;

impl MigrateElsaToBiarritzRc1 for DotMovement {
	async fn migrate_elsa_to_biarritz_rc1(&self) -> Result<Config, ElsaToBiarritzRc1Error> {
		// get the value
		let value = self
			.try_load_value()
			.await
			.map_err(|e| ElsaToBiarritzRc1Error::MigrationFailed(e.into()))?;

		// migrate the value
		let migrated_config = ElsaToBiarritzRc1::migrate(value)?;

		// write the migrated value
		self.try_overwrite_config_to_json(&migrated_config)
			.map_err(|e| ElsaToBiarritzRc1Error::MigrationFailed(e.into()))?;

		Ok(migrated_config)
	}
}
