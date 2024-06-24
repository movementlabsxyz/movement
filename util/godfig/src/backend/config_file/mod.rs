use anyhow::Context;
use flocks::tfrwlock::TfrwLock;
use std::sync::Arc;
use std::fs::File;
use crate::backend::BackendOperations;
use async_stream::stream;
use futures::Stream;
use std::io::{Read, Write};
use std::io::Seek;

#[derive(Clone)]
pub struct ConfigFile {
    pub (crate) lock: Arc<TfrwLock<File>>,
    pub (crate) polling_interval: std::time::Duration,
}

impl ConfigFile {
    
    pub fn new(file: File) -> Self {
        Self {
            lock: Arc::new(TfrwLock::new(file)),
            polling_interval: std::time::Duration::from_millis(20),
        }
    }

    pub fn with_polling_interval(mut self, interval: std::time::Duration) -> Self {
        self.polling_interval = interval;
        self
    }

}

impl BackendOperations for ConfigFile {
    async fn try_get<K, T>(&self, key: K) -> Result<Option<T>, anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned,
    {
        let contents = {
            let mut write_guard = self.lock.write().await?;
            let mut contents = String::new();
            write_guard.seek(std::io::SeekFrom::Start(0))?;
            write_guard.read_to_string(&mut contents)?;
            contents
        };

        let json: serde_json::Value = serde_json::from_str(&contents)?;
        let keys = key.into();
        let mut current = &json;
        for k in keys {
            if current.get(&k).is_none() {
                return Ok(None);
            }
            current = &current[&k];
        }
        let result = serde_json::from_value(current.clone())?;
        Ok(Some(result))
    }

    async fn try_set<K, T>(&self, key: K, value: Option<T>) -> Result<(), anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::Serialize,
    {
        // here we want to hold the write lock for the duration of the function
        let mut write_guard = self.lock.write().await?;
        let mut contents = String::new();
        write_guard.seek(std::io::SeekFrom::Start(0))?;
        write_guard.read_to_string(&mut contents).context(
            "Failed to read contents of file"
        )?; // read the contents of the file (if any

        let mut json: serde_json::Value = if contents.is_empty() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&contents)?
        };

        let keys = key.into();
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

        // serialize the contents and write to the file
        contents = serde_json::to_string(&json)?;
        write_guard.seek(std::io::SeekFrom::Start(0))?;
        write_guard.write_all(contents.as_bytes())?;
        write_guard.flush()?;

        Ok(())
    }

    async fn try_wait_for<K, T>(&self, key: K) -> Result<T, anyhow::Error>
    where
        K: Into<Vec<String>> + Send,
        T: serde::de::DeserializeOwned,
    {
        let key_clone = key.into();
        loop {
            if let Ok(Some(result)) = self.try_get(key_clone.clone()).await {
                return Ok(result);
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    async fn try_stream<K, T>(&self, key: K) -> Result<impl Stream<Item = Result<Option<T>, anyhow::Error>>, anyhow::Error>
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
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        })
    }
}


#[cfg(test)]
pub mod test {
    use super::*;

    #[tokio::test]
    async fn test_locking() -> Result<(), anyhow::Error> {
        let file = File::create("test.txt")?;
        let config_file = ConfigFile::new(file);

        // cannot read and write at the same time
        let _write_guard = config_file.lock.write().await?;
        // trying to acquire a read guard should now fail
        let read_result = tokio::time::timeout(std::time::Duration::from_millis(100), config_file.lock.read()).await;

        assert!(read_result.is_err(), "Read lock should not be acquired while holding write lock");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_set() -> Result<(), anyhow::Error> {
        let file = tempfile::tempfile()?;
        let config_file = ConfigFile::new(file);

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
        let config_file = ConfigFile::new(file);

        // start one thread that will wait for the value
        let config_file_clone = config_file.clone();
        let wait_task = tokio::spawn(async move {
            let result = config_file_clone.try_wait_for::<_, i32>(vec!["key".to_string()]).await?;
            assert_eq!(result, 42);
            Ok::<(), anyhow::Error>(())
        });

        // start another thread that will set the value
        let config_file_clone = config_file.clone();
        let set_task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            config_file_clone.try_set(vec!["key".to_string()], Some(42)).await?;
            Ok::<(), anyhow::Error>(())
        });

        // wait for both tasks to finish
        tokio::try_join!(wait_task, set_task)?;

        Ok(())

    }

}