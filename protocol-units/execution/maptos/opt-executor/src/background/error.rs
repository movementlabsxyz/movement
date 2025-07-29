use movement_signer_loader::LoaderError;
use thiserror::Error;

/// Domain error for the executor background task.
#[derive(Debug, Error)]
pub enum Error {
	#[error("internal error: {0}")]
	InternalError(String),
	#[error("mempool client request stream closed")]
	InputClosed,
	#[error("Error during sign loading: {0}")]
	LoadSign(#[from] LoaderError),
	#[error("Error serialization of transaction batch: {0}")]
	SerialisationFailed(#[from] bcs::Error),
}

impl From<anyhow::Error> for Error {
	fn from(e: anyhow::Error) -> Self {
		Error::InternalError(e.to_string())
	}
}
