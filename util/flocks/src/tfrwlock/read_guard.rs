use std::ops::Deref;
use tokio::sync::RwLockReadGuard;
use rustix::fd::AsFd;
use crate::frwlock::{FrwLock, FrwLockReadGuard, FrwLockError};

pub struct TfrwLockReadGuard<'a, T: AsFd> {
    _outer: RwLockReadGuard<'a, FrwLock<T>>,
    inner: Option<FrwLockReadGuard<'a, T>>,
}

impl<'a, T: AsFd> TfrwLockReadGuard<'a, T> {
    pub(crate) fn new(mut outer: RwLockReadGuard<'a, FrwLock<T>>) -> Result<Self, FrwLockError> {
        let inner = outer.try_read().ok().map(|inner_guard| Self {
            _outer: outer,
            inner: Some(inner_guard),
        });
        match inner {
            Some(guard) => Ok(guard),
            None => Err(FrwLockError::LockNotAvailable),
        }
    }
}

impl<'a, T: AsFd> Deref for TfrwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.inner.as_ref().unwrap()
    }
}