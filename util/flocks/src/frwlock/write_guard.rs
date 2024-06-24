use std::ops::{Deref, DerefMut};
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};

pub struct FrwLockWriteGuard<T: AsFd> {
    pub(crate) data: *mut T
}

impl<T: AsFd> Deref for FrwLockWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: AsFd> DerefMut for FrwLockWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl <T: AsFd> Drop for FrwLockWriteGuard<T> {
    fn drop(&mut self) {
        flock(
            unsafe { &*self.data },
    FlockOperation::Unlock).expect("Failed to unlock file");
    }
}