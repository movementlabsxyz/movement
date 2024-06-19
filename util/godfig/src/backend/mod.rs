pub(crate) trait BackendInternalOperations {
    /// Acquires a--usually time-limited--lock on a key.
    async fn try_acquire<K>(&self, key: K) -> Result<(), anyhow::Error>
    where
        K: Into<String> + Send;

    /// Sets a key-value pair.
    async fn try_set_unsafe<K, T>(&self, key: K, value: T) -> Result<(), anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::Serialize;

    /// Gets a value from a key.
    async fn try_get_unsafe<K, T>(&self, key: K) -> Result<T, anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::de::DeserializeOwned;

    /// Releases a lock on a key.
    async fn try_release<K>(&self, key: K) -> Result<(), anyhow::Error>
    where
        K: Into<String> + Send;
}

pub trait BackendOperations: BackendInternalOperations {
    /// Tries to get a value from a key.
    async fn try_get<K, T>(&self, key: K) -> Result<T, anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::de::DeserializeOwned,
    {
        let key_str = key.into();
        self.try_acquire(key_str.clone()).await?;
        let value = self.try_get_unsafe(key_str.clone()).await?;
        self.try_release(key_str).await?;
        Ok(value)
    }

    /// Tries to set a key-value pair.
    async fn try_set<K, T>(&self, key: K, value: T) -> Result<(), anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::Serialize,
    {
        let key_str = key.into();
        self.try_acquire(key_str.clone()).await?;
        self.try_set_unsafe(key_str.clone(), value).await?;
        self.try_release(key_str).await?;
        Ok(())
    }

    /// Tries to stream values from a key.
    async fn try_stream<K, T>(&self, key: K) -> Result<impl futures::Stream<Item = T>, anyhow::Error>
    where
        K: Into<String> + Send,
        T: serde::de::DeserializeOwned;
}
