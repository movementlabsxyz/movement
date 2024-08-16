use godfig::env_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_block_retry_count")]
	pub block_retry_count: u64,

	#[serde(default = "default_block_retry_increment_microseconds")]
	pub block_retry_increment_microseconds: u64,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			block_retry_count: default_block_retry_count(),
			block_retry_increment_microseconds: default_block_retry_increment_microseconds(),
		}
	}
}

env_default!(default_block_retry_count, "BLOCK_RETRY_COUNT", u64, 10);

env_default!(
	default_block_retry_increment_microseconds,
	"BLOCK_RETRY_INCREMENT_MICROSECONDS",
	u64,
	5000
);
