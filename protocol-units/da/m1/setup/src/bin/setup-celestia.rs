use celestia_rpc::HeaderClient;
use m1_da_light_node_setup::local::{Local, M1DaLightNodeSetupOperations};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
    let config = m1_da_light_node_util::Config::try_from_env_toml_file()?;

    let local = Local::new();
    let config = local.setup(dot_movement, config).await?;

	Ok(())
}