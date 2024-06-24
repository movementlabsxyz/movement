use m1_da_light_node::v1::{LightNodeV1, LightNodeV1Operations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// TODO: set up tracing-subscriber if the "logging" feature is enabled

	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement
		.try_get_config_from_json::<m1_da_light_node_util::config::M1DaLightNodeConfig>()?;

	let light_node = LightNodeV1::try_from_config(config.m1_da_light_node_config).await?;

	// log out the node's configuration with tracing
	tracing::info!("{:?}", light_node);

	light_node.run().await?;

	Ok(())
}
