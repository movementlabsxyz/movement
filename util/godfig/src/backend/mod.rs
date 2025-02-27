pub mod config_file;

use flocks::tfrwlock::FileRwLockError;
use futures::Stream;
use std::future::Future;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GodfigBackendError {
	#[error("An error occurs during config deserialization: {0}")]
	ConfigDeserializationError(String),
	#[error("Backend Error: {0}")]
	BackendError(#[from] anyhow::Error),
	#[error("IO Error: {0}")]
	IOError(#[from] std::io::Error),
	// any other error
	#[error("Error: {0}")]
	Error(String),
}

impl From<serde_json::Error> for GodfigBackendError {
	fn from(error: serde_json::Error) -> Self {
		GodfigBackendError::BackendError(error.into())
	}
}

impl From<FileRwLockError> for GodfigBackendError {
	fn from(error: FileRwLockError) -> Self {
		GodfigBackendError::BackendError(error.into())
	}
}

pub trait BackendOperations {
	fn try_get<K, T>(&self, key: K) -> impl Future<Output = Result<Option<T>, GodfigBackendError>>
	where
		K: Into<Vec<String>> + Send,
		T: serde::de::DeserializeOwned;

	fn try_set<K, T>(
		&self,
		key: K,
		value: Option<T>,
	) -> impl Future<Output = Result<(), GodfigBackendError>>
	where
		K: Into<Vec<String>> + Send,
		T: serde::Serialize;

	fn try_wait_for<K, T>(&self, key: K) -> impl Future<Output = Result<T, GodfigBackendError>>
	where
		K: Into<Vec<String>> + Send,
		T: serde::de::DeserializeOwned;

	fn try_stream<K, T>(
		&self,
		key: K,
	) -> impl Future<
		Output = Result<
			impl Stream<Item = Result<Option<T>, GodfigBackendError>>,
			GodfigBackendError,
		>,
	>
	where
		K: Into<Vec<String>> + Send,
		T: serde::de::DeserializeOwned + serde::Serialize;

	fn try_transaction<K, T, F, Fut>(
		&self,
		key: K,
		callback: F,
	) -> impl Future<Output = Result<(), GodfigBackendError>>
	where
		K: Into<Vec<String>> + Send,
		T: serde::de::DeserializeOwned + serde::Serialize + Send,
		F: FnOnce(Option<T>) -> Fut + Send,
		Fut: std::future::Future<Output = Result<Option<T>, GodfigBackendError>> + Send;

	fn try_transaction_with_result<K, T, R, F, Fut>(
		&self,
		key: K,
		callback: F,
	) -> impl Future<Output = Result<R, GodfigBackendError>>
	where
		K: Into<Vec<String>> + Send,
		T: serde::de::DeserializeOwned + serde::Serialize + Send,
		F: FnOnce(Option<T>) -> Fut + Send,
		Fut: std::future::Future<Output = Result<(Option<T>, R), GodfigBackendError>> + Send;
}
