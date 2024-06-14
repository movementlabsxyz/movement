use anyhow::Context;
use monza_full_node::{partial::MonzaPartialNode, MonzaFullNode};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	#[cfg(feature = "logging")]
	{
		use tracing_subscriber::EnvFilter;

		tracing_subscriber::fmt()
			.with_env_filter(
				EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
			)
			.init();
	}

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let path = dot_movement.get_path().join("config.toml");
	let config = monza_config::Config::try_from_toml_file(&path).unwrap_or_default();
	let executor = MonzaPartialNode::try_from_config(config)
		.await
		.context("Failed to create the executor")?;

	executor.run().await.context("Failed to run the executor")?;

	Ok(())
}
