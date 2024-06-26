use std::ops::Deref;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};

pub struct FrwLockReadGuard<T: AsFd> {
    pub(crate) data: *const T
}

impl<T: AsFd> Deref for FrwLockReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: AsFd> Drop for FrwLockReadGuard<T> {
    fn drop(&mut self) {
        flock(
            unsafe { &*self.data }, 
            FlockOperation::Unlock).expect("Failed to unlock file");
    }
}