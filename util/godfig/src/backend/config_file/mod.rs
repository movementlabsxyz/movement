use flocks::tfrwlock::{FileRwLock, FileRwLockWriteGuard};
use std::sync::Arc;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}
};

use crate::backend::{BackendOperations, GodfigBackendError};
use async_stream::stream;
use futures::Stream;
use std::future::Future;
use serde::{
    Serialize,
    de::DeserializeOwned
};
use std::time::Duration;

#[derive(Clone)]
pub struct ConfigFile {
    pub (crate) lock: Arc<FileRwLock<File>>,
    pub (crate) polling_interval: Duration,
}

impl ConfigFile {
    
    pub fn new(file: File) -> Self {
        Self {
            lock: Arc::new(FileRwLock::new(file)),
            polling_interval: Duration::from_millis(20),
        }
    }

    pub fn with_polling_interval(mut self, interval: Duration) -> Self {
        self.polling_interval = interval;
        self
    }

    async fn try_get_with_guard<K, T>(mut write_guard : FileRwLockWriteGuard<'_, File>, key: K) -> Result<(Option<T>, FileRwLockWriteGuard<'_, File>), GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned,
    {
        let mut contents = String::new();
        write_guard.seek(std::io::SeekFrom::Start(0)).await?;
        write_guard.read_to_string(&mut contents).await?;
        if contents.is_empty() {
            return Ok((None, write_guard));
        }
        
        let json: serde_json::Value = serde_json::from_str(&contents).map_err(
            |e| GodfigBackendError::TypeContractMismatch(e.to_string())
        )?;

        let keys = key.into();
        let mut current = &json;
        for k in keys {
            if current.get(&k).is_none() {
                return Ok((None, write_guard));
            }
            current = &current[&k];
        }
        let result = serde_json::from_value(current.clone())?;
        Ok((Some(result), write_guard))

    }

    async fn try_set_with_guard<K, T>(mut write_guard : FileRwLockWriteGuard<'_, File>, key: K, value: Option<T>) -> Result<FileRwLockWriteGuard<'_, File>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::Serialize,
    {
        let mut contents = String::new();
        // write_guard.seek(std::io::SeekFrom::Start(0)).await?;
        write_guard.read_to_string(&mut contents).await?;
        let mut json: serde_json::Value = if contents.is_empty() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&contents)?// parse the contents as JSON (if any
        };

        let keys = key.into();

        if keys.is_empty() {
            // handle the case with 0 keys, setting the top-level JSON
            json = match value {
                Some(v) => serde_json::to_value(v)?,
                None => serde_json::Value::Null,
            };
        } else {
            // handle the case with keys
            let mut current = &mut json;
            for k in &keys[..keys.len() - 1] {
                if current.get_mut(k).is_none() {
                    current[k] = serde_json::Value::Object(serde_json::Map::new());
                }
                current = current.get_mut(k).unwrap();
            }
            let last_key = keys[keys.len() - 1].clone();
    
            // set or unset the value
            match value {
                Some(v) => {
                    current[last_key] = serde_json::to_value(v)?;
                },
                None => {
                    current.as_object_mut().ok_or(
                        anyhow::anyhow!("Cannot set a value on a non-object")
                    )?.remove(&last_key);
                },
            }
        }
    
        // serialize the contents and write to the file
        contents = serde_json::to_string_pretty(&json)?;
        write_guard.seek(std::io::SeekFrom::Start(0)).await?;
        write_guard.write_all(contents.as_bytes()).await?;
        write_guard.flush().await?;
    
        Ok(write_guard)
    }

}

impl BackendOperations for ConfigFile {
    async fn try_get<K, T>(&self, key: K) -> Result<Option<T>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned,
    {
        let write_guard = self.lock.write().await?;
        let (value, guard) = Self::try_get_with_guard(write_guard, key).await?;
        Ok(value)
    }

    async fn try_set<K, T>(&self, key: K, value: Option<T>) -> Result<(), GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::Serialize,
    {
        let write_guard = self.lock.write().await?;
        Self::try_set_with_guard(write_guard, key, value).await?;

        Ok(())
    }

    async fn try_wait_for<K, T>(&self, key: K) -> Result<T, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned,
    {
        let key_clone = key.into();
        loop {
            if let Ok(Some(result)) = self.try_get(key_clone.clone()).await {
                return Ok(result);
            }
            tokio::time::sleep(self.polling_interval).await;
        }
    }

    async fn try_stream<K, T>(&self, key: K) -> Result<impl Stream<Item = Result<Option<T>, GodfigBackendError>>, GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned + serde::Serialize,
    {
        let key_clone = key.into();
        let mut last: Option<Vec<u8>> = None;
        Ok(stream! {
            loop {
                if let Ok(result) = self.try_get(key_clone.clone()).await {
                    let serialized_result = serde_json::to_vec(&result)?;

                    if last.as_ref().map_or(true, |last| *last != serialized_result) {
                        last = Some(serialized_result);
                        yield Ok(result);
                    }
                }
                tokio::time::sleep(self.polling_interval).await;
            }
        })
    }

    async fn try_transaction<K, T, F, Fut>(&self, key: K, callback: F) -> Result<(), GodfigBackendError>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned + serde::Serialize + Send,
        F: FnOnce(Option<T>) -> Fut + Send,
        Fut: Future<Output = Result<Option<T>, GodfigBackendError>> + Send {

        let key = key.into();
    
        // obtain the write_guard which will be held for the duration of the function
        let mut write_guard = self.lock.write().await?;
    
        // get the current value
        let (current_value, mut write_guard) = Self::try_get_with_guard(write_guard, key.clone()).await?;

        let new_value = callback(current_value).await?;

        // set the new value
        write_guard = Self::try_set_with_guard(write_guard, key, new_value).await?;

        Ok(())

    }

    async fn try_transaction_with_result<K, T, R, F, Fut>(&self, key: K, callback: F) -> Result<R, GodfigBackendError>
        where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned + serde::Serialize + Send,
        F: FnOnce(Option<T>) -> Fut + Send,
        Fut: Future<Output = Result<(Option<T>, R), GodfigBackendError>> + Send {


        let key = key.into();
    
        // obtain the write_guard which will be held for the duration of the function
        let mut write_guard = self.lock.write().await?;
    
        // get the current value
        let (current_value, mut write_guard) = Self::try_get_with_guard(write_guard, key.clone()).await?;

        let (new_value, result) = callback(current_value).await?;

        // set the new value
        write_guard = Self::try_set_with_guard(write_guard, key, new_value).await?;

        Ok(result)

    }

}


#[cfg(test)]
pub mod test {
    use super::*;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct TestConfig {
        pub key: String,
        pub value: i32,
    }

    #[tokio::test]
    async fn test_locking() -> Result<(), anyhow::Error> {
        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file.into());

        // cannot read and write at the same time
        let _write_guard = config_file.lock.write().await?;
        // trying to acquire a read guard should now fail
        let read_result = tokio::time::timeout(Duration::from_millis(100), config_file.lock.read()).await;

        assert!(read_result.is_err(), "Read lock should not be acquired while holding write lock");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_set() -> Result<(), anyhow::Error> {
        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file.into());

        // set a value
        config_file.try_set(vec!["key".to_string()], Some(42)).await?;
        // get the value
        let result = config_file.try_get::<_, i32>(vec!["key".to_string()]).await?;
        assert_eq!(result, Some(42));

        Ok(())
    }

    #[tokio::test]
    async fn test_wait_for() -> Result<(), anyhow::Error> {

        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file.into());

        // start one thread that will wait for the value
        let config_file_clone = config_file.clone();
        let wait_task = tokio::spawn(async move {
            let result = config_file_clone.try_wait_for::<_, i32>(vec!["key".to_string()]).await?;
            assert_eq!(result, 42);
            Ok::<(), GodfigBackendError>(())
        });

        // start another thread that will set the value
        let config_file_clone = config_file.clone();
        let set_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            config_file_clone.try_set(vec!["key".to_string()], Some(42)).await?;
            Ok::<(), GodfigBackendError>(())
        });

        // wait for both tasks to finish
        tokio::try_join!(wait_task, set_task)?;

        Ok(())

    }

    #[tokio::test]
    async fn test_transaction() -> Result<(), anyhow::Error> {
        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file.into());

        // set a value
        config_file.try_set(vec!["key".to_string()], Some(42)).await?;

        // increment the value
        config_file.try_transaction(vec!["key".to_string()], |value| async move {
            Ok(value.map(|v : i32| v + 1))
        }).await?;

        // increment the value again
        config_file.try_transaction(vec!["key".to_string()], |value| async move {
            Ok(value.map(|v : i32| v + 1))
        }).await?;

        // check the value
        let result = config_file.try_get::<_, i32>(vec!["key".to_string()]).await?;
        assert_eq!(result, Some(44));

        Ok(())
    }

    #[tokio::test]
    async fn test_transaction_with_result() -> Result<(), anyhow::Error> {
        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file.into());

        // set a value
        config_file.try_set(vec!["key".to_string()], Some(42)).await?;

        // increment the value
        let result = config_file.try_transaction_with_result(vec!["key".to_string()], |value| async move {
            Ok((value.map(|v : i32| v + 1), "result".to_string()))
        }).await?;

        assert_eq!(result, "result");

        let result = config_file.try_get::<_, i32>(vec!["key".to_string()]).await?;
        assert_eq!(result, Some(43));

        Ok(())
    }

    #[tokio::test]
    async fn test_struct() -> Result<(), anyhow::Error> {
        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file.into());

        // set a value
        config_file.try_set(vec!["key".to_string()], Some(TestConfig {
            key: "test".to_string(),
            value: 42,
        })).await?;

        // get the value
        let result = config_file.try_get::<_, TestConfig>(vec!["key".to_string()]).await?;
        assert_eq!(result, Some(TestConfig {
            key: "test".to_string(),
            value: 42,
        }));

        Ok(())
    }

}