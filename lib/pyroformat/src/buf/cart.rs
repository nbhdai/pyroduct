//! [`PyroCart`] — a `{Struct}Ref<'a>` paired with the buffer it borrows from,
//! with no lifetime parameter of its own. See `docs/deep-ref-plan.md` §6.
//!
//! A `FooRef<'a>` cannot outlive the buffer it points into, which makes it
//! useless as a return value, a channel payload, or a struct field.
//! `PyroCart<Foo>` is the owning pair: `Send + Sync`, hands out a `FooRef` on
//! demand via [`PyroCart::get`].

use crate::buf::error::PyroError;
use crate::buf::header::PyroHeader;
use crate::buf::{PyroBuf, PyroView};
use crate::{Layout, Reader};

/// The owner half of a [`PyroCart`]: either handle type has a stable payload
/// address (see `PyroCart`'s doc comment for why `PyroVec` is excluded — it
/// reallocates on growth).
pub enum CartOwner {
    Buf(PyroBuf),
    View(PyroView),
}

impl CartOwner {
    pub fn payload(&self) -> &[u8] {
        match self {
            CartOwner::Buf(b) => b.payload(),
            CartOwner::View(v) => v.payload(),
        }
    }

    fn check_schema(&self, expected: u64) -> Result<(), (u64, u64)> {
        match self {
            CartOwner::Buf(b) => b.check_schema(expected),
            CartOwner::View(v) => v.check_schema(expected),
        }
    }
}

/// # Safety
/// - `Self::Ref<'a>` (from [`Reader`]) must be covariant in `'a`.
/// - `Self::Ref<'a>` must have no `Drop` impl that dereferences the borrow.
///
/// `#[derive(PyroCodec)]` emits this; do not implement it by hand without
/// checking both.
pub unsafe trait CovariantRef: Reader {}

/// A `T::Ref<'_>` paired with the buffer it borrows from. No lifetime
/// parameter, `Send + Sync` (when the ref is), and cheap to move.
pub struct PyroCart<T: CovariantRef + 'static> {
    /// Borrows from `owner`'s payload. Declared first so it drops first.
    /// The `'static` is a lie erased at construction and re-narrowed by
    /// `get`.
    view: T::Ref<'static>,
    owner: CartOwner,
}

unsafe impl<T: CovariantRef + Reader + 'static> Send for PyroCart<T> where T::Ref<'static>: Send {}
unsafe impl<T: CovariantRef + Reader + 'static> Sync for PyroCart<T> where T::Ref<'static>: Sync {}

impl<T: CovariantRef + Reader + Layout + 'static> PyroCart<T> {
    /// Validates the schema hash against `T`'s, then reads.
    pub fn from_buf(buf: PyroBuf) -> Result<Self, PyroError> {
        Self::read(CartOwner::Buf(buf))
    }

    pub fn from_view(view: PyroView) -> Result<Self, PyroError> {
        Self::read(CartOwner::View(view))
    }

    fn read(owner: CartOwner) -> Result<Self, PyroError> {
        let expected = <T as Layout>::SCHEMA_HASH;
        owner
            .check_schema(expected)
            .map_err(|(expected, got)| PyroError::schema_mismatch(expected, got))?;

        // SAFETY: `owner`'s payload has a stable address (see `CartOwner`'s
        // doc comment) and outlives `Self` by drop order (`view` before
        // `owner`), so a borrow of it may be stored with an erased
        // `'static` lifetime and re-narrowed on every access through `get`.
        let payload: &'static [u8] =
            unsafe { std::mem::transmute::<&[u8], &'static [u8]>(owner.payload()) };
        let view: T::Ref<'static> = T::pyro_read(payload)?;

        Ok(Self { view, owner })
    }
}

impl<T: CovariantRef + 'static> PyroCart<T> {
    /// Narrows the erased `'static` borrow back down to `&self`'s lifetime.
    /// Sound exactly because `CovariantRef` guarantees `T::Ref<'a>` is
    /// covariant in `'a` and carries no borrow-reading `Drop`.
    pub fn get(&self) -> &T::Ref<'_> {
        unsafe { std::mem::transmute::<&T::Ref<'static>, &T::Ref<'_>>(&self.view) }
    }

    pub fn payload(&self) -> &[u8] {
        self.owner.payload()
    }

    pub fn owner(&self) -> &CartOwner {
        &self.owner
    }

    pub fn into_owner(self) -> CartOwner {
        self.owner
    }

    /// `PyroView` is refcounted and `Clone`; `PyroBuf` is not. Returns `None`
    /// for a `Buf`-backed cart rather than requiring `T::Ref<'static>: Clone`
    /// to build a `Clone` impl that would panic on half its inputs.
    pub fn try_clone(&self) -> Option<Self>
    where
        T::Ref<'static>: Clone,
    {
        match &self.owner {
            CartOwner::View(v) => Some(Self {
                view: self.view.clone(),
                owner: CartOwner::View(v.clone()),
            }),
            CartOwner::Buf(_) => None,
        }
    }
}
