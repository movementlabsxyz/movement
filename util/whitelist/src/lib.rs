pub mod file;
use thiserror::Error;

/// Domain error for the whitelist
#[derive(Debug, Error)]
pub enum Error {
	#[error("prevalidator internal error: {0}")]
	Internal(String),
}

#[tonic::async_trait]
pub trait WhitelistOperations<T> {
	async fn is_whitelisted(&self, item: &T) -> Result<bool, Error>;
}
