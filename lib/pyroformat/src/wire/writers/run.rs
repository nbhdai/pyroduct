//! Containers the pre writers can walk: anything that hands out its elements
//! as one contiguous slice.
//!
//! Which container holds a level makes no difference on the wire, so this is
//! the only place containers appear. The nesting itself is walked by
//! [`Nest`](super::nesting::Nest), over whatever `Run`s it is given.

use std::{borrow::Cow, rc::Rc, sync::Arc};

/// A container whose elements can be borrowed as one slice.
///
/// Strings and byte buffers are runs too, of `u8`; they live in `basic.rs`.
pub trait Run {
    type Item;

    fn as_run(&self) -> &[Self::Item];
}

impl<T> Run for [T] {
    type Item = T;

    #[inline]
    fn as_run(&self) -> &[T] {
        self
    }
}

impl<T> Run for Vec<T> {
    type Item = T;

    #[inline]
    fn as_run(&self) -> &[T] {
        self
    }
}

impl<T> Run for Box<[T]> {
    type Item = T;

    #[inline]
    fn as_run(&self) -> &[T] {
        self
    }
}

impl<T> Run for Arc<[T]> {
    type Item = T;

    #[inline]
    fn as_run(&self) -> &[T] {
        self
    }
}

impl<T> Run for Rc<[T]> {
    type Item = T;

    #[inline]
    fn as_run(&self) -> &[T] {
        self
    }
}

impl<T: Clone> Run for Cow<'_, [T]> {
    type Item = T;

    #[inline]
    fn as_run(&self) -> &[T] {
        self
    }
}

impl<R: Run + ?Sized> Run for &R {
    type Item = R::Item;

    #[inline]
    fn as_run(&self) -> &[R::Item] {
        (**self).as_run()
    }
}
