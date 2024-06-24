pub mod config_file;

use futures::Stream;

pub trait BackendOperations {
    async fn try_get<K, T>(&self, key: K) -> Result<Option<T>, anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned;

    async fn try_set<K, T>(&self, key: K, value: Option<T>) -> Result<(), anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::Serialize;

    async fn try_wait_for<K, T>(&self, key: K) -> Result<T, anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned;

    async fn try_stream<K, T>(&self, key: K) -> Result<impl Stream<Item = Result<Option<T>, anyhow::Error>>, anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned + serde::Serialize;
}