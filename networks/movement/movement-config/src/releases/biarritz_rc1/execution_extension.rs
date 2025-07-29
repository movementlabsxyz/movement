use godfig::env_default;
use serde::{Deserialize, Serialize};

/// The execution extension configuration.
/// This covers Movement configurations that do not configure the Maptos executor, but do configure the way it is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	/// The number of times to retry a block if it fails to execute.
	#[serde(default = "default_block_retry_count")]
	pub block_retry_count: u64,

	/// The amount by which to increment the block timestamp if it fails to execute. (This is the most common reason for a block to fail to execute.)
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

env_default!(default_block_retry_count, "SUZUKA_BLOCK_RETRY_COUNT", u64, 10);

env_default!(
	default_block_retry_increment_microseconds,
	"SUZUKA_BLOCK_RETRY_INCREMENT_MICROSECONDS",
	u64,
	5000
);
