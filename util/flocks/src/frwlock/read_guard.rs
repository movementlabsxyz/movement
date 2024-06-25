use std::ops::Deref;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};
use std::marker::PhantomData;

pub struct FrwLockReadGuard<'a, T: AsFd> {
    pub(crate) data: *const T,
    pub(crate) _marker: PhantomData<&'a T>
}

impl<T: AsFd> Deref for FrwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: AsFd> Drop for FrwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        flock(
            unsafe { &*self.data }, 
            FlockOperation::Unlock).expect("Failed to unlock file");
    }
}