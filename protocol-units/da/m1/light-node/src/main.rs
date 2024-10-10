use m1_da_light_node::v1::{LightNodeV1, Manager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let tracing_config = movement_tracing::Config::from_env()?;
	let _guard = movement_tracing::init_tracing_subscriber(
		env!("CARGO_BIN_NAME"),
		env!("CARGO_PKG_VERSION"),
		&tracing_config,
	)?;

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_path = dot_movement.get_config_json_path();
	let config_file = tokio::fs::File::open(config_path).await?;
	let manager = Manager::<LightNodeV1>::new(config_file).await?;
	manager.try_run().await?;

	Ok(())
}
