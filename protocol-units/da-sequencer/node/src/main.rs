#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Define da-sequencer config path
	let dot_movement = dot_movement::DotMovement::try_from_env()?;

	movement_da_sequencer_node::start(dot_movement).await
}
