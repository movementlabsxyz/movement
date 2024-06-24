pub mod config_file;

/// Backend trait for key-value storage.
pub trait BackendOperations {
    /// Tries to get a value from a key.
    async fn try_get<K, T>(&self, key: K) -> Result<T, anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::de::DeserializeOwned;

    /// Tries to set a key-value pair.
    async fn try_set<K, T>(&self, key: K, value: T) -> Result<(), anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::Serialize;

    /// Try wait for a key to be set.
    async fn try_wait_for<K, T>(&self, key: K) -> Result<T, anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::de::DeserializeOwned;

    /// Tries to stream values from a key.
    async fn try_stream<K, T>(&self, key: K) -> Result<impl futures::Stream<Item = T>, anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::de::DeserializeOwned;
}
