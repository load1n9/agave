// ported from https://github.com/theseus-os/Theseus/blob/theseus_main/libs/dereffer/src/lib.rs

#![no_std]
#![feature(const_mut_refs)]

use core::ops::{Deref, DerefMut};

pub struct DerefsTo<Inner, Ref: ?Sized> {
    inner: Inner,
    deref_func: fn(&Inner) -> &Ref,
}
impl<Inner, Ref: ?Sized> DerefsTo<Inner, Ref> {
    pub const fn new(inner: Inner, deref_func: fn(&Inner) -> &Ref) -> Self {
        Self { inner, deref_func }
    }
}
impl<Inner, Ref> DerefsTo<Inner, Ref>
where
    Inner: Deref<Target = Ref>,
    Ref: ?Sized,
{
    pub const fn new_default(inner: Inner) -> Self {
        Self {
            inner,
            deref_func: Deref::deref,
        }
    }
}
impl<Inner, Ref: ?Sized> Deref for DerefsTo<Inner, Ref> {
    type Target = Ref;
    fn deref(&self) -> &Self::Target {
        (self.deref_func)(&self.inner)
    }
}

pub struct DerefsToMut<Inner, Ref: ?Sized> {
    inner: DerefsTo<Inner, Ref>,
    deref_mut_func: fn(&mut Inner) -> &mut Ref,
}
impl<Inner, Ref: ?Sized> DerefsToMut<Inner, Ref> {
    pub const fn new(
        inner: Inner,
        deref_func: fn(&Inner) -> &Ref,
        deref_mut_func: fn(&mut Inner) -> &mut Ref,
    ) -> Self {
        Self {
            inner: DerefsTo::new(inner, deref_func),
            deref_mut_func,
        }
    }
}
impl<Inner, Ref> DerefsToMut<Inner, Ref>
where
    Inner: DerefMut<Target = Ref>,
    Ref: ?Sized,
{
    pub const fn new_default(inner: Inner) -> Self {
        Self {
            inner: DerefsTo::new(inner, Deref::deref),
            deref_mut_func: DerefMut::deref_mut,
        }
    }
}
impl<Inner, Ref: ?Sized> Deref for DerefsToMut<Inner, Ref> {
    type Target = Ref;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<Inner, Ref: ?Sized> DerefMut for DerefsToMut<Inner, Ref> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        (self.deref_mut_func)(&mut self.inner.inner)
    }
}
