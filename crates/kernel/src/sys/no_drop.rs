// ported from https://github.com/theseus-os/Theseus/blob/theseus_main/kernel/no_drop/src/lib.rs

use core::{
    fmt::{self, Debug},
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

#[repr(transparent)]
pub struct NoDrop<T>(ManuallyDrop<T>);

impl<T> NoDrop<T> {
    pub const fn new(obj: T) -> NoDrop<T> {
        NoDrop(ManuallyDrop::new(obj))
    }

    pub const fn into_inner(self) -> T {
        ManuallyDrop::into_inner(self.0)
    }
}

impl<T: Debug> Debug for NoDrop<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0.deref(), f)
    }
}
impl<T> Deref for NoDrop<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for NoDrop<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
