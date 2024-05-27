use suzuka_config::{Config, ConfigError};

#[tokio::main]
async fn main() -> Result<(), ConfigError> {
	// read any values from env, but populate the default values if they are not present
	let config = Config::try_from_env()?;
	// write the values to the env
	print!("{}", config.write_bash_export_string()?);
	Ok(())
}

