pub mod read_guard;
pub mod write_guard;

pub use write_guard::FrwLockWriteGuard;
pub use read_guard::FrwLockReadGuard;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd,
};
use std::cell::UnsafeCell;
use thiserror::Error;
use tokio::task::yield_now;

#[derive(Debug, Error)]
pub enum FrwLockError {
    #[error("Lock is not available")]
    LockNotAvailable,
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
}

/// A file-based read-write lock.
/// This only mutually excludes processes trying to violate the lock, not the same process--which is not considered contention.
/// If you want to prevent contention within the same process, you should wrap this in your preferred synchronization primitive.
pub struct FrwLock<T: AsFd> {
    cell: UnsafeCell<T>,
}

impl<T: AsFd> FrwLock<T> {
    pub fn new(file: T) -> Self {
        Self {
            cell: UnsafeCell::new(file),
        }
    }

    pub(crate) fn try_write(&self) -> Result<FrwLockWriteGuard<'_, T>, FrwLockError> {
        let file = unsafe { &*self.cell.get() };
        match flock(file, FlockOperation::NonBlockingLockExclusive) {
            Ok(_) => {
                Ok(FrwLockWriteGuard {
                    data: self.cell.get(),
                    _marker: std::marker::PhantomData,
                })
            },
            Err(rustix::io::Errno::WOULDBLOCK) => Err(FrwLockError::LockNotAvailable),
            Err(e) => Err(FrwLockError::FileError(e.into())),
        }
    }

    pub(crate) fn try_read(&self) -> Result<FrwLockReadGuard<'_, T>, FrwLockError> {
        let file = unsafe { &*self.cell.get() };
        match flock(file, FlockOperation::NonBlockingLockShared) {
            Ok(_) => {
                Ok(FrwLockReadGuard {
                    data: self.cell.get(),
                    _marker: std::marker::PhantomData,
                })
            },
            Err(rustix::io::Errno::WOULDBLOCK) => Err(FrwLockError::LockNotAvailable),
            Err(e) => Err(FrwLockError::FileError(e.into())),
        }
    }

    pub async fn write(&self) -> Result<FrwLockWriteGuard<'_, T>, FrwLockError> {
        loop {
            match self.try_write() {
                Ok(guard) => return Ok(guard),
                Err(FrwLockError::LockNotAvailable) => {
                    // Yield control to other tasks
                    yield_now().await;
                },
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn read(&self) -> Result<FrwLockReadGuard<'_, T>, FrwLockError> {
        loop {
            match self.try_read() {
                Ok(guard) => return Ok(guard),
                Err(FrwLockError::LockNotAvailable) => {
                    // Yield control to other tasks
                    yield_now().await;
                },
                Err(e) => return Err(e),
            }
        }
    }

}

// As long as T: Send + Sync, it's fine to send and share FrwLock<T> between threads.
// If T were not Send, sending and sharing a FrwLock<T> would be bad, since you can access T through
// FrwLock<T>.
unsafe impl<T> Send for FrwLock<T> where T: AsFd + Sized + Send {}
unsafe impl<T> Sync for FrwLock<T> where T: AsFd + Sized + Send + Sync {}
// NB: These impls need to be explicit since we're storing a raw pointer.
// Safety: Stores a raw pointer to `T`, so if `T` is `Sync`, the lock guard over
// `T` is `Send`.
unsafe impl<T> Send for FrwLockReadGuard<'_, T> where T: AsFd + Sized + Sync {}
unsafe impl<T> Sync for FrwLockReadGuard<'_, T> where T: AsFd + Sized + Send + Sync {}
unsafe impl<T> Send for FrwLockWriteGuard<'_, T> where T: AsFd + Sized + Sync {}
unsafe impl<T> Sync for FrwLockWriteGuard<'_, T> where T: AsFd + Sized + Send + Sync {}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, Write};

    use super::*;
    use tempfile::tempfile;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_frwlock_basic_uncontested() -> Result<(), anyhow::Error> {
        let file = tempfile()?;
        let frwlock = FrwLock::new(file);

        // get a write lock and use it
        {
            let mut write_guard = frwlock.write().await?;
            write_guard.write_all(b"hello world")?;
        }

        // use write lock to read the data
        {
            let mut write_guard = frwlock.write().await?;
            let mut buf = Vec::new();
            write_guard.seek(std::io::SeekFrom::Start(0))?;
            write_guard.read_to_end(&mut buf)?;
            assert_eq!(buf, b"hello world");
        }

        // get a read lock and use it
        {
            let read_guard = frwlock.read().await?;
            read_guard.metadata()?;
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn test_within_process_contested() -> Result<(), anyhow::Error> {
        let file = tempfile()?;
        let frwlock = FrwLock::new(file);

        // no exclusion within the thread
        {
            let write_guard = frwlock.write().await?;

            /// This should be fine
            let read_guard = frwlock.try_read()?;

            /// This should also be fine 
            let read_guard = frwlock.read().await?;
        }

        // now, we will wrap the lock in a FrwLock to make sure we can't have contention within the same process
        let rwlock = RwLock::new(frwlock);
        {
            let mut write_guard = rwlock.write().await;
            let _frw_write_guard = write_guard.write().await?;

            /// This should fail
           match rwlock.try_write() {
                // Unfortunately, Tokio lock error enum members are private, so we can't match on them.
                Ok(_) => panic!("Should not be able to get a write lock"),
                Err(_) => (),
           }

        }

        Ok(())
    }

    #[tokio::test]
    pub async fn test_works_with_buf_writer_and_reader() -> Result<(), anyhow::Error> {
        let file = tempfile()?;
        let frwlock = FrwLock::new(file);

        // get a write lock and use it
        {
            let mut write_guard = frwlock.write().await?;
            let mut writer = std::io::BufWriter::new(&mut *write_guard);
            writer.write_all(b"hello world")?;
            writer.flush()?;
        }

        // use write lock to read the data
        {
            let mut write_guard = frwlock.write().await?;
            let mut reader = std::io::BufReader::new(&mut *write_guard);
            let mut buf = Vec::new();
            reader.seek(std::io::SeekFrom::Start(0))?;
            reader.read_to_end(&mut buf)?;
            assert_eq!(buf, b"hello world");
        }

        Ok(())
    }

}