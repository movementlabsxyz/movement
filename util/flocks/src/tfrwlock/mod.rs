pub mod write_guard;
pub mod read_guard;

pub use write_guard::TfrwLockWriteGuard;
pub use read_guard::TfrwLockReadGuard;

use tokio::sync::RwLock;
use thiserror::Error;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd,
};
use tokio::task::yield_now;

#[derive(Debug, Error)]
pub enum TfrwLockError {
    #[error("Lock is not available")]
    LockNotAvailable,
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
}

impl From<tokio::sync::TryLockError> for TfrwLockError {
    fn from(e: tokio::sync::TryLockError) -> Self {
        TfrwLockError::LockNotAvailable
    }
}

/// Wraps a file-based read-write lock in a Tokio-friendly interface.
pub struct TfrwLock<T: AsFd> {
    lock : RwLock<T>
}

impl<T: AsFd> TfrwLock<T> {
    
    pub fn new(file: T) -> Self {
        Self {
            lock: RwLock::new(file)
        }
    }

    pub(crate) fn try_write(&self) -> Result<TfrwLockWriteGuard<'_, T>, TfrwLockError> {

        let (res, write) = {
            let file = self.lock.try_write()?;
            (
                flock(&*file, FlockOperation::NonBlockingLockExclusive),
                file
            )
        };

        match res {
            Ok(_) => {
                Ok(TfrwLockWriteGuard {
                    guard : write
                })
            },
            Err(rustix::io::Errno::WOULDBLOCK) => Err(TfrwLockError::LockNotAvailable),
            Err(e) => Err(TfrwLockError::FileError(e.into())),
        }

    }

    pub(crate) fn try_read(&self) -> Result<TfrwLockReadGuard<'_, T>, TfrwLockError> {
        
        let (res, read) = {
            let file = self.lock.try_read()?;
            (
                flock(&*file, FlockOperation::NonBlockingLockShared),
                file
            )
        };

        match res {
            Ok(_) => {
                Ok(TfrwLockReadGuard {
                    guard : read
                })
            },
            Err(rustix::io::Errno::WOULDBLOCK) => Err(TfrwLockError::LockNotAvailable),
            Err(e) => Err(TfrwLockError::FileError(e.into())),
        }

    }

    pub async fn write(&self) -> Result<TfrwLockWriteGuard<'_, T>, TfrwLockError> {
        loop {
            match self.try_write() {
                Ok(guard) => return Ok(guard),
                Err(TfrwLockError::LockNotAvailable) => {
                    // Yield control to other tasks
                    yield_now().await;
                },
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn read(&self) -> Result<TfrwLockReadGuard<'_, T>, TfrwLockError> {
        loop {
            match self.try_read() {
                Ok(guard) => return Ok(guard),
                Err(TfrwLockError::LockNotAvailable) => {
                    // Yield control to other tasks
                    yield_now().await;
                },
                Err(e) => return Err(e),
            }
        }
    }

}


#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, Write};

    use super::*;
    use tempfile::tempfile;

    #[tokio::test]
    async fn test_tfrwlock_basic_uncontested() -> Result<(), anyhow::Error> {
        let file = tempfile()?;
        let tfrwlock = TfrwLock::new(file);

        // get a write lock and use it
        {
            let mut write_guard = tfrwlock.write().await?;
            write_guard.write_all(b"hello world")?;
        }

        // use write lock to read the data
        {
            let mut write_guard = tfrwlock.write().await?;
            let mut buf = Vec::new();
            write_guard.seek(std::io::SeekFrom::Start(0))?;
            write_guard.read_to_end(&mut buf)?;
            assert_eq!(buf, b"hello world");
        }

        // get a read lock and use it
        {
            let read_guard = tfrwlock.read().await?;
            read_guard.metadata()?;
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn test_within_process_contested() -> Result<(), anyhow::Error> {
        let file = tempfile()?;
        let tfrwlock = TfrwLock::new(file);

        // exclusion within the thread
        let write_guard = tfrwlock.write().await?;

        /// This should be fine
        let err = tfrwlock.try_read().err().ok_or(anyhow::Error::msg("Expected error"))?;
        match err {
            TfrwLockError::LockNotAvailable => (),
            _ => panic!("Expected LockNotAvailable")
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn test_works_with_buf_writer_and_reader() -> Result<(), anyhow::Error> {
        let file = tempfile()?;
        let tfrwlock = TfrwLock::new(file);

        // get a write lock and use it
        {
            let mut write_guard = tfrwlock.write().await?;
            let mut writer = std::io::BufWriter::new(&mut *write_guard);
            writer.write_all(b"hello world")?;
            writer.flush()?;
        }

        // use write lock to read the data
        {
            let mut write_guard = tfrwlock.write().await?;
            let mut reader = std::io::BufReader::new(&mut *write_guard);
            let mut buf = Vec::new();
            reader.seek(std::io::SeekFrom::Start(0))?;
            reader.read_to_end(&mut buf)?;
            assert_eq!(buf, b"hello world");
        }

        Ok(())
    }

}