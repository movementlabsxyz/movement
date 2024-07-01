use std::ops::Deref;
use tokio::sync::RwLockReadGuard;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};

pub struct FileRwLockReadGuard<'a, T: AsFd> {
    pub(crate) guard: RwLockReadGuard<'a, T>,
}

impl<T: AsFd> Deref for FileRwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<T: AsFd> Drop for FileRwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        flock(
            &*self.guard,
            FlockOperation::Unlock,
        ).expect("Failed to unlock file");
        // self.guard drops here
    }
}