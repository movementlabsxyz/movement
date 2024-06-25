use std::ops::{Deref, DerefMut};
use tokio::sync::RwLockWriteGuard;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};

pub struct TfrwLockWriteGuard<'a, T: AsFd> {
    pub (crate) guard: RwLockWriteGuard<'a, T>
}

impl<T: AsFd> Deref for TfrwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<T: AsFd> DerefMut for TfrwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

impl<T: AsFd> Drop for TfrwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        flock(
            &*self.guard,
            FlockOperation::Unlock,
        ).expect("Failed to unlock file");
        // self.guard drops here
    }
}