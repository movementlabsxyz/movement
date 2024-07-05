pub mod config_file;

use thiserror::Error;
use flocks::tfrwlock::FileRwLockError;
use futures::Stream;
use std::future::Future;
use serde::{
    Serialize,
    de::DeserializeOwned
};

#[derive(Debug, Error)]
pub enum GodfigBackendError {
    #[error("Type Contract Mismatch")]
    TypeContractMismatch(String),
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
    async fn try_get<K, T>(&self, key: K) -> Result<Option<T>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: DeserializeOwned;

    async fn try_set<K, T>(&self, key: K, value: Option<T>) -> Result<(), GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: Serialize;

    async fn try_wait_for<K, T>(&self, key: K) -> Result<T, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: DeserializeOwned;

    async fn try_stream<K, T>(&self, key: K) -> Result<impl Stream<Item = Result<Option<T>, GodfigBackendError>>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: DeserializeOwned + Serialize;

    /// This function is used to perform a transaction on the backend. The callback function is called with the current value of the key.
    /// The callback function should return a future that resolves to the new value of the key.
    /// `K` is the key.
    /// `T` is the type of the data that will be stored in the backend.
    /// `F` is the callback function that will be called with the current value of the key.
    /// `Fut` is the future that the callback function should return.
    async fn try_transaction<K, T, F, Fut>(&self, key: K, callback: F) -> Result<(), GodfigBackendError>
        where
        K: Into<Vec<String>> + Send,
        T: DeserializeOwned + Serialize + Send,
        F: FnOnce(Option<T>) -> Fut + Send,
        Fut: Future<Output = Result<Option<T>, GodfigBackendError>> + Send;

    /// This function is used to perform a transaction on the backend. The callback function is called with the current value of the key.
    /// The callback function should return a future that resolves to a tuple of the new value and the result of the transaction.
    /// `K` is the key.
    /// `T` is the type of the data that will be stored in the backend.
    /// `R` is the result of the transaction.
    /// `F` is the callback function that will be called with the current value of the key.
    /// `Fut` is the future that the callback function should return.
    async fn try_transaction_with_result<K, T, R, F, Fut>(&self, key: K, callback: F) -> Result<R, GodfigBackendError>
        where
        K: Into<Vec<String>> + Send,
        T: DeserializeOwned + Serialize + Send,
        F: FnOnce(Option<T>) -> Fut + Send,
        Fut: Future<Output = Result<(Option<T>, R), GodfigBackendError>> + Send;
}
