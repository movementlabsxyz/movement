pub mod read_guard;
pub mod write_guard;

pub use write_guard::TfrwLockWriteGuard;
pub use read_guard::TfrwLockReadGuard;

use tokio::sync::RwLock;
use rustix::fd::AsFd;
use thiserror::Error;
use crate::frwlock::{FrwLock, FrwLockError};

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

pub struct TfrwLock<T: AsFd> {
    lock: RwLock<FrwLock<T>>,
}

impl<T: AsFd> TfrwLock<T> {
    pub fn new(file: T) -> Self {
        Self {
            lock: RwLock::new(FrwLock::new(file)),
        }
    }

    pub async fn write<'a>(&'a self) -> Result<TfrwLockWriteGuard<'a, T>, TfrwLockError> {
        let outer_guard = self.lock.write().await;
        TfrwLockWriteGuard::new(outer_guard).map_err(|e| e.into())
    }

    pub async fn read<'a>(&'a self) -> Result<TfrwLockReadGuard<'a, T>, TfrwLockError> {
        let outer_guard = self.lock.read().await;
        TfrwLockReadGuard::new(outer_guard).map_err(|e| e.into())
    }
}