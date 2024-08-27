use m1_da_light_node::v1::{LightNodeV1, Manager};

use std::env;

const TIMING_LOG_ENV: &str = "M1_DA_LIGHT_NODE_TIMING_LOG";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let tracing_config =
		movement_tracing::Config { timing_log_path: env::var_os(TIMING_LOG_ENV).map(Into::into) };
	let _guard = movement_tracing::init_tracing_subscriber(tracing_config);

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_path = dot_movement.get_config_json_path();
	let config_file = tokio::fs::File::open(config_path).await?;
	let manager = Manager::<LightNodeV1>::new(config_file).await?;
	manager.try_run().await?;

	Ok(())
}
