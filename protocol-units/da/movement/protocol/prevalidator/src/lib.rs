pub mod aptos;

use thiserror::Error;

/// Domain error for the transaction pipe task
#[derive(Debug, Error)]
pub enum Error {
	#[error("prevalidator internal error: {0}")]
	Internal(String),
	#[error("prevalidator validation error: {0}")]
	Validation(String),
}
