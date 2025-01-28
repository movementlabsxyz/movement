use crate::{ElsaToBiarritzRc1, ElsaToBiarritzRc1Error, MigrateElsaToBiarritzRc1};
use dot_movement::DotMovement;
use maptos_framework_release_util::ReleaseSigner;

impl MigrateElsaToBiarritzRc1 for DotMovement {
	async fn migrate_framework_from_elsa_to_biarritz_rc1(
		&self,
	) -> Result<(), ElsaToBiarritzRc1Error> {
		// get the movement config from dot movement

		Ok(())
	}
}
