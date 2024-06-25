use std::ops::{Deref, DerefMut};
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd
};
use std::marker::PhantomData;

pub struct FrwLockWriteGuard<'a, T: AsFd> {
    pub(crate) data: *mut T,
    pub(crate) _marker: PhantomData<&'a mut T>
}

impl<T: AsFd> Deref for FrwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: AsFd> DerefMut for FrwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl <T: AsFd> Drop for FrwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        flock(
            unsafe { &*self.data },
    FlockOperation::Unlock).expect("Failed to unlock file");
    }
}