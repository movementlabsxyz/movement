pub mod config_file;

use thiserror::Error;
use flocks::tfrwlock::TfrwLockError;
use futures::Stream;

#[derive(Debug, Error)]
pub enum GodfigBackendError {
    #[error("Type Contract Mismatch")]
    TypeContractMismatch(String),
    #[error("Backend Error: {0}")]
    BackendError(#[from] anyhow::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}

impl From<serde_json::Error> for GodfigBackendError {
    fn from(error: serde_json::Error) -> Self {
        GodfigBackendError::BackendError(error.into())
    }
}

impl From<TfrwLockError> for GodfigBackendError {
    fn from(error: TfrwLockError) -> Self {
        GodfigBackendError::BackendError(error.into())
    }
}

pub trait BackendOperations {
    async fn try_get<K, T>(&self, key: K) -> Result<Option<T>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned;

    async fn try_set<K, T>(&self, key: K, value: Option<T>) -> Result<(), GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::Serialize;

    async fn try_wait_for<K, T>(&self, key: K) -> Result<T, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned;

    async fn try_stream<K, T>(&self, key: K) -> Result<impl Stream<Item = Result<Option<T>, GodfigBackendError>>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned + serde::Serialize;

    async fn try_transaction<K, T, F, Fut>(&self, key: K, callback: F) -> Result<(), GodfigBackendError>
        where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned + serde::Serialize + Send,
        F: FnOnce(Option<T>) -> Fut + Send,
        Fut: std::future::Future<Output = Result<Option<T>, GodfigBackendError>> + Send;
}