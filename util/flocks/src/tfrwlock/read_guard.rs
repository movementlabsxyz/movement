use std::ops::Deref;
use tokio::sync::RwLockReadGuard;
use rustix::fd::AsFd;
use crate::frwlock::{FrwLock, FrwLockReadGuard};

pub struct TfrwLockReadGuard<'a, T: AsFd> {
    _outer: RwLockReadGuard<'a, FrwLock<T>>,
    inner: FrwLockReadGuard<T>,
}

impl<'a, T: AsFd> TfrwLockReadGuard<'a, T> {
    pub(crate) fn new(outer: RwLockReadGuard<'a, FrwLock<T>>, inner: FrwLockReadGuard<T>) -> Self {
        Self { _outer : outer, inner }
    }
}

impl<'a, T: AsFd> Deref for TfrwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}