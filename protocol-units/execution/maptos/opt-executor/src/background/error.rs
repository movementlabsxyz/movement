use thiserror::Error;

/// Domain error for the executor background task.
#[derive(Debug, Clone, Error)]
pub enum Error {
	#[error("internal error: {0}")]
	InternalError(String),
	#[error("mempool client request stream closed")]
	InputClosed,
}

impl From<anyhow::Error> for Error {
	fn from(e: anyhow::Error) -> Self {
		Error::InternalError(e.to_string())
	}
}
