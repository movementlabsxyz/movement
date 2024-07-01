use std::ops::{Deref, DerefMut};
use tokio::sync::RwLockWriteGuard;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};

pub struct FileRwLockWriteGuard<'a, T: AsFd> {
    pub (crate) guard: RwLockWriteGuard<'a, T>
}

impl<T: AsFd> Deref for FileRwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<T: AsFd> DerefMut for FileRwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

impl<T: AsFd> Drop for FileRwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        flock(
            &*self.guard,
            FlockOperation::Unlock,
        ).expect("Failed to unlock file");
        // self.guard drops here
    }
}