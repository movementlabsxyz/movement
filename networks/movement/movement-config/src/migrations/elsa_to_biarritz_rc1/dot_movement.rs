use crate::migrations::elsa_to_biarritz_rc1::{
	ElsaToBiarritzRc1, ElsaToBiarritzRc1Error, MigrateElsaToBiarritzRc1,
};
use crate::releases::biarritz_rc1::Config;
use dot_movement::DotMovement;

impl MigrateElsaToBiarritzRc1 for DotMovement {
	async fn migrate_elsa_to_biarritz_rc1(&self) -> Result<Config, ElsaToBiarritzRc1Error> {
		let value = self
			.try_load_value()
			.await
			.map_err(|e| ElsaToBiarritzRc1Error::MigrationFailed(e.into()))?;
		Ok(ElsaToBiarritzRc1::migrate(value)?)
	}
}
