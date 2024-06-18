use dot_movement::DotMovement;
use m1_da_light_node_util::Config;

pub mod celestia_appd;
pub mod celestia_bridge;

pub trait Runner {
	async fn run(&self, dot_movement: &DotMovement, config: Config) -> Result<(), anyhow::Error>;
}
