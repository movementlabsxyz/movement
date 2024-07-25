use m1_da_light_node::v1::{LightNodeV1, Manager};

use tracing_subscriber::{filter::LevelFilter, fmt::format::FmtSpan, EnvFilter};

use std::env;

const TIMING_ENV_VAR: &str = "M1_DA_LIGHT_NODE_TIMING";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	init_tracing_subscriber();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_path = dot_movement.get_config_json_path();
	let config_file = tokio::fs::File::open(config_path).await?;
	let manager = Manager::<LightNodeV1>::new(config_file).await?;
	manager.try_run().await?;

	Ok(())
}

fn init_tracing_subscriber() {
	// TODO: compose console_subscriber as a layer
	let env_filter = EnvFilter::builder()
		.with_default_directive(LevelFilter::INFO.into())
		.from_env_lossy();
	let mut subscriber = tracing_subscriber::fmt().with_env_filter(env_filter);
	if env::var(TIMING_ENV_VAR).map_or(false, |v| v != "0") {
		subscriber = subscriber.with_span_events(FmtSpan::CLOSE);
	}
	subscriber.init()
}
