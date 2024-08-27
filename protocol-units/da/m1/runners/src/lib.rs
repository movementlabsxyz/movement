pub mod celestia_appd;
pub mod celestia_bridge;
pub mod celestia_light;
use m1_da_light_node_util::config::M1DaLightNodeConfig;

pub trait Runner {
	async fn run(
		&self,
		dot_movement: dot_movement::DotMovement,
		config: M1DaLightNodeConfig,
	) -> Result<(), anyhow::Error>;
}
