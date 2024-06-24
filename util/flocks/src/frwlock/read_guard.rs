use std::marker::PhantomData;
use std::ops::Deref;
use rustix::{
    fs::{flock, FlockOperation},
    fd::AsFd,
};

pub struct FrwLockReadGuard<'a, T: AsFd> {
    pub(crate) data: *const T,
    pub(crate) _marker: PhantomData<&'a T>, // Ensuring lifetime and immutability semantics.
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
            FlockOperation::Unlock,
        ).expect("Failed to unlock file");
    }
}
