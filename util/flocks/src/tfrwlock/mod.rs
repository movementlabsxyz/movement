pub mod write_guard;
pub mod read_guard;

pub use write_guard::TfrwLockWriteGuard;
pub use read_guard::TfrwLockReadGuard;

use tokio::sync::RwLock;
use crate::frwlock::{FrwLock, FrwLockError};
use rustix::fd::AsFd;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum TfrwLockError {
    #[error("Lock is not available")]
    LockNotAvailable,
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
}

impl From<FrwLockError> for TfrwLockError {
    fn from(e: FrwLockError) -> Self {
        match e {
            FrwLockError::LockNotAvailable => TfrwLockError::LockNotAvailable,
            FrwLockError::FileError(e) => TfrwLockError::FileError(e),
        }
    }
}

/// A file-based read-write lock.
/// This only mutually excludes processes trying to violate the lock, not the same process--which is not considered contention.
/// If you want to prevent contention within the same process, you should wrap this in your preferred synchronization primitive.
pub struct TfrwLock<T: AsFd> {
    lock : RwLock<FrwLock<T>>
}

impl<T: AsFd> TfrwLock<T> {
    pub fn new(file: T) -> Self {
        Self {
            lock: RwLock::new(FrwLock::new(file))
        }
    }

    pub async fn write(&self) -> Result<TfrwLockWriteGuard<T>, TfrwLockError> {
        let outer_guard = self.lock.write().await;
        let inner_guard = outer_guard.write().await?;
        Ok(TfrwLockWriteGuard::new(outer_guard, inner_guard))
    }

    pub async fn read(&self) -> Result<TfrwLockReadGuard<T>, TfrwLockError> {
        let outer_guard = self.lock.read().await;
        let inner_guard = outer_guard.read().await?;
        Ok(TfrwLockReadGuard::new(outer_guard, inner_guard))
    }

}