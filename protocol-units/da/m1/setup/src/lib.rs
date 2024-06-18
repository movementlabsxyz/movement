use dot_movement::DotMovement;
use m1_da_light_node_util::Config;

pub mod local;

pub trait M1DaLightNodeSetupOperations {
	async fn setup(
		&self,
		dot_movement: &DotMovement,
		config: Config,
	) -> Result<Config, anyhow::Error>;
}
