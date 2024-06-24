use tokio::sync::RwLockWriteGuard;
use rustix::fd::AsFd;
use crate::frwlock::{FrwLock, FrwLockWriteGuard, FrwLockError};
use std::ops::{Deref, DerefMut};


pub struct TfrwLockWriteGuard<'a, T: AsFd> {
    _outer: RwLockWriteGuard<'a, FrwLock<T>>,
    inner: Option<FrwLockWriteGuard<'a, T>>,
}

impl<'a, T: AsFd> TfrwLockWriteGuard<'a, T> {
    pub(crate) fn new(mut outer: RwLockWriteGuard<'a, FrwLock<T>>) -> Result<Self, FrwLockError> {
        let inner = outer.try_write().ok().map(|inner_guard| Self {
            _outer: outer,
            inner: Some(inner_guard),
        });
        match inner {
            Some(guard) => Ok(guard),
            None => Err(FrwLockError::LockNotAvailable),
        }
    }
}

impl<'a, T: AsFd> Deref for TfrwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.inner.as_ref().unwrap()
    }
}

impl<'a, T: AsFd> DerefMut for TfrwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner.as_mut().unwrap()
    }
}