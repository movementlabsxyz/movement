pub mod celestia_appd;
pub mod celestia_bridge;
pub mod celestia_light;
use movement_celestia_da_util::config::CelestiaDaLightNodeConfig;

pub trait Runner {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: CelestiaDaLightNodeConfig,
	) -> Result<(), anyhow::Error>;
}
