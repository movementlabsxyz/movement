use m1_da_light_node_setup::{
    M1DaLightNodeSetupOperations,
    local::Local,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
    let path = dot_movement.get_path().join("config.toml");
    let config = m1_da_light_node_util::Config::try_from_toml_file(
        &path
    ).unwrap_or_default();

    let local = Local::new();
    let config = local.setup(dot_movement, config).await?;
    config.try_write_to_toml_file(&path)?;

	Ok(())
}