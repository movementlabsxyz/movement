use nix::fcntl::{flock, FlockArg};
use std::fs::File;
use std::path::PathBuf;
use std::os::unix::io::AsRawFd;
use std::time::Duration;
use tokio::time::sleep;
use serde::{Serialize, de::DeserializeOwned};
use anyhow::Error;

#[derive(Debug, Clone)]
pub struct ConfigFile {
    path: PathBuf,
    lock: Option<FileLock<File>>,
}

impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path, lock: None }
    }

    async fn acquire_lock(&mut self) -> Result<(), Error> {
        let file = File::open(&self.path)?;
        loop {
            match FileLock::try_acquire(file.try_clone()?) {
                Ok(lock) => {
                    self.lock = Some(lock);
                    return Ok(());
                },
                Err(e) if e.to_string() == "Lock is not available" => {
                    sleep(Duration::from_secs(1)).await;
                },
                Err(e) => return Err(e),
            }
        }
    }
}

struct FileLock<T> {
    file: T,
}

impl FileLock<File> {
    fn new(file: File) -> Self {
        Self { file }
    }

    fn try_acquire(file: File) -> Result<Self, Error> {
        match flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
            Ok(_) => Ok(Self::new(file)),
            Err(nix::errno::Errno::EWOULDBLOCK) => Err(anyhow::anyhow!("Lock is not available")),
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }
}

impl<T> Drop for FileLock<T>
where
    T: AsRawFd,
{
    fn drop(&mut self) {
        let _ = flock(self.file.as_raw_fd(), FlockArg::Unlock);
    }
}

pub trait BackendInternalOperations {
    async fn try_acquire<K>(&mut self, key: K) -> Result<(), Error>
    where
        K: Into<String> + Send;

    async fn try_set_unsafe<K, T>(&mut self, key: K, value: T) -> Result<(), Error>
    where
        K: Into<String> + Send,
        T: Serialize;

    async fn try_get_unsafe<K, T>(&mut self, key: K) -> Result<T, Error>
    where
        K: Into<String> + Send,
        T: DeserializeOwned;

    async fn try_release<K>(&mut self, key: K) -> Result<(), Error>
    where
        K: Into<String> + Send;
}

impl BackendInternalOperations for ConfigFile {
    async fn try_acquire<K>(&mut self, _key: K) -> Result<(), Error>
    where
        K: Into<String> + Send,
    {
        self.acquire_lock().await
    }

    async fn try_set_unsafe<K, T>(&mut self, _key: K, _value: T) -> Result<(), Error>
    where
        K: Into<String> + Send,
        T: Serialize,
    {
        if self.lock.is_none() {
            self.acquire_lock().await?;
        }
        // Implement your serialization and writing to file logic here
        Ok(())
    }

    async fn try_get_unsafe<K, T>(&mut self, _key: K) -> Result<T, Error>
    where
        K: Into<String> + Send,
        T: DeserializeOwned + Default,
    {
        if self.lock.is_none() {
            self.acquire_lock().await?;
        }
        // Implement your deserialization and reading from file logic here
        Ok(Default::default())
    }

    async fn try_release<K>(&mut self, _key: K) -> Result<(), Error>
    where
        K: Into<String> + Send,
    {
        self.lock = None;
        Ok(())
    }
}

impl<T: AsRawFd> FileLock<T> {
    fn try_acquire_shared(file: T) -> Result<Self, Error> {
        match flock(file.as_raw_fd(), FlockArg::LockSharedNonblock) {
            Ok(_) => Ok(Self { file }),
            Err(nix::errno::Errno::EWOULDBLOCK) => Err(anyhow::anyhow!("Lock is not available")),
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    async fn acquire_shared(file: T) -> Result<Self, Error> {
        loop {
            match Self::try_acquire_shared(file.as_raw_fd().try_clone()?) {
                Ok(lock) => return Ok(lock),
                Err(e) if e.to_string() == "Lock is not available" => {
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl ConfigFile {
    async fn acquire_shared_lock(&mut self) -> Result<(), Error> {
        let file = File::open(&self.path)?;
        loop {
            match FileLock::try_acquire_shared(file.try_clone()?) {
                Ok(lock) => {
                    self.lock = Some(lock);
                    return Ok(());
                },
                Err(e) if e.to_string() == "Lock is not available" => {
                    sleep(Duration::from_secs(1)).await;
                },
                Err(e) => return Err(e),
            }
        }
    }
}

#[async_trait]
impl BackendOperations for ConfigFile {
    async fn read<K, T>(&mut self, _key: K) -> Result<T, Error>
    where
        K: Into<String> + Send,
        T: DeserializeOwned + Default,
    {
        self.acquire_shared_lock().await?;
        // Implement your deserialization and reading from file logic here
        Ok(Default::default())
    }

    async fn write<K, T>(&mut self, _key: K, _value: T) -> Result<(), Error>
    where
        K: Into<String> + Send,
        T: Serialize,
    {
        self.acquire_lock().await?;
        // Implement your serialization and writing to file logic here
        Ok(())
    }
}
