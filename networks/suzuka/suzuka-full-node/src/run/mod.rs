use crate::node::manager::Manager;

use std::env;

use crate::common_args::MovementArgs;
use clap::Parser;

const TIMING_LOG_ENV: &str = "SUZUKA_TIMING_LOG";

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Runs the Suzuka Full Node")]
pub struct Run {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Run {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let tracing_config = movement_tracing::Config {
			timing_log_path: env::var_os(TIMING_LOG_ENV).map(Into::into),
		};
		let _guard = movement_tracing::init_tracing_subscriber(tracing_config);

		// get the config file
		let dot_movement = self.movement_args.dot_movement()?;
		let config_file = dot_movement.try_get_or_create_config_file().await?;

		let manager = Manager::new(config_file).await?;
		manager.try_run().await?;

		Ok(())
	}
}
