use m1_da_light_node::v1::{Manager, LightNodeV1};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// TODO: set up tracing-subscriber if the "logging" feature is enabled

	// console_subscriber::init();
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_path = dot_movement.get_config_json_path();
	let config_file = tokio::fs::File::open(config_path).await?;
	let manager = Manager::<LightNodeV1>::new(config_file).await?;
	manager.try_run().await?;

	Ok(())
}
