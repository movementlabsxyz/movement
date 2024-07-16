use serde::{Deserialize, Serialize};
use godfig::env_short_default;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {

    #[serde(default = "default_gas_limit")]
	pub gas_limit: u64,
	/// Timeout for batching blocks, in milliseconds
	#[serde(default = "default_batch_timeout")]
	pub batch_timeout: u64,
	#[serde(default = "default_transaction_send_retries")]
	pub transaction_send_retries: u32,
}

env_short_default!(
    default_gas_limit,
    u64,
    10_000_000_000 as u64
);

env_short_default!(
    default_batch_timeout,
    u64,
    2000 as u64
);

env_short_default!(
    default_transaction_send_retries,
    u32,
    10 as u32
);

impl Default for Config {
    fn default() -> Self {
        Config {
            gas_limit: default_gas_limit(),
            batch_timeout: default_batch_timeout(),
            transaction_send_retries: default_transaction_send_retries(),
        }
    }
}