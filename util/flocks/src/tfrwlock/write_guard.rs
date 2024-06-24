use std::ops::{Deref, DerefMut};
use tokio::sync::RwLockWriteGuard;
use rustix::fd::AsFd;
use crate::frwlock::{FrwLock, FrwLockWriteGuard};

pub struct TfrwLockWriteGuard<'a, T: AsFd> {
    _outer: RwLockWriteGuard<'a, FrwLock<T>>,
    inner: FrwLockWriteGuard<T>,
}

impl<'a, T: AsFd> TfrwLockWriteGuard<'a, T> {
    pub(crate) fn new(outer: RwLockWriteGuard<'a, FrwLock<T>>, inner: FrwLockWriteGuard<T>) -> Self {
        Self { _outer : outer, inner }
    }
}

impl<'a, T: AsFd> Deref for TfrwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'a, T: AsFd> DerefMut for TfrwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}
