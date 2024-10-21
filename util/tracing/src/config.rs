use anyhow::anyhow;
use std::env;

const OTLP_TRACING_ENV: &str = "MOVEMENT_OTLP";

/// Options for tracing configuration.
#[derive(Debug, Default)]
pub struct Config {
	/// URL of the OpenTelemetry collector endpoint using the OTLP gRPC protocol.
	/// If the value is `None`, telemetry is not exported.
	pub otlp_grpc_url: Option<String>,
}

impl Config {
	/// Get the tracing configuration from well-known environment variables.
	pub fn from_env() -> Result<Self, anyhow::Error> {
		let otlp_grpc_url = match env::var(OTLP_TRACING_ENV) {
			Ok(url) => Some(url),
			Err(env::VarError::NotPresent) => None,
			Err(env::VarError::NotUnicode(s)) => {
				return Err(anyhow!(
					"value of environment variable {OTLP_TRACING_ENV} is not valid UTF-8: {}",
					s.to_string_lossy()
				));
			}
		};
		Ok(Self { otlp_grpc_url })
	}
}
