use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

/// A wrapper around a raw non-null `*mut T` that indicates that the possessor
/// of this wrapper owns the referent. This in turn implies that the
/// `Unique<T>` is `Send`/`Sync` if `T` is `Send`/`Sync`, unlike a raw
/// `*mut T` (which conveys no particular ownership semantics).  It
/// also implies that the referent of the pointer should not be
/// modified without a unique path to the `Unique` reference. Useful
/// for building abstractions like `Vec<T>` or `Box<T>`, which
/// internally use raw pointers to manage the memory that they own.
// #[allow(missing_debug_implementations)]
// #[unstable(feature = "unique", reason = "needs an RFC to flesh out design",
//            issue = "27730")]
pub struct Unique<T: ?Sized> {
    // pointer: NonZero<*const T>,
    pointer: *const T,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

/// `Unique` pointers are `Send` if `T` is `Send` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
// #[unstable(feature = "unique", issue = "27730")]
unsafe impl<T: Send + ?Sized> Send for Unique<T> { }

/// `Unique` pointers are `Sync` if `T` is `Sync` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
// #[unstable(feature = "unique", issue = "27730")]
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> { }

// #[unstable(feature = "unique", issue = "27730")]
impl<T: ?Sized> Unique<T> {
    /// Creates a new `Unique`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    pub /*const*/ unsafe fn new(ptr: *mut T) -> Unique<T> {
        // Unique { pointer: NonZero::new(ptr), _marker: PhantomData }
        Unique { pointer: ptr, _marker: PhantomData }
    }

    /// Dereferences the content.
    #[allow(unused)]
    pub unsafe fn get(&self) -> &T {
        &*self.pointer
    }

    #[allow(unused)]
    /// Mutably dereferences the content.
    pub unsafe fn get_mut(&mut self) -> &mut T {
        &mut ***self
    }
}

// #[unstable(feature = "unique", issue = "27730")]
// impl<T: ?Sized, U: ?Sized> CoerceUnsized<Unique<U>> for Unique<T> where T: Unsize<U> { }

// #[unstable(feature = "unique", issue= "27730")]
impl<T:?Sized> Deref for Unique<T> {
    type Target = *mut T;

    #[inline]
    fn deref(&self) -> &*mut T {
        unsafe { mem::transmute(&self.pointer) }
    }
}

// #[unstable(feature = "unique", issue = "27730")]
impl<T> fmt::Pointer for Unique<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.pointer, f)
    }
}

/// A wrapper around a raw non-null `*mut T` that indicates that the possessor
/// of this wrapper has shared ownership of the referent. Useful for
/// building abstractions like `Rc<T>` or `Arc<T>`, which internally
/// use raw pointers to manage the memory that they own.
// #[allow(missing_debug_implementations)]
// #[unstable(feature = "shared", reason = "needs an RFC to flesh out design",
//            issue = "27730")]
pub struct Shared<T: ?Sized> {
    // pointer: NonZero<*const T>,
    pointer: *const T,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

// /// `Shared` pointers are not `Send` because the data they reference may be aliased.
// NB: This impl is unnecessary, but should provide better error messages.
// #[unstable(feature = "shared", issue = "27730")]
// impl<T: ?Sized> !Send for Shared<T> { }

// /// `Shared` pointers are not `Sync` because the data they reference may be aliased.
// NB: This impl is unnecessary, but should provide better error messages.
// #[unstable(feature = "shared", issue = "27730")]
// impl<T: ?Sized> !Sync for Shared<T> { }

// #[unstable(feature = "shared", issue = "27730")]
impl<T: ?Sized> Shared<T> {
    /// Creates a new `Shared`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    pub unsafe fn new(ptr: *mut T) -> Self {
        // Shared { pointer: NonZero::new(ptr), _marker: PhantomData }
        Shared { pointer: ptr, _marker: PhantomData }
    }
}

// #[unstable(feature = "shared", issue = "27730")]
impl<T: ?Sized> Shared<T> {
    /// Acquires the underlying pointer as a `*mut` pointer.
    pub unsafe fn as_mut_ptr(&self) -> *mut T {
        **self as _
    }
}

// #[unstable(feature = "shared", issue = "27730")]
impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// #[unstable(feature = "shared", issue = "27730")]
impl<T: ?Sized> Copy for Shared<T> { }

// #[unstable(feature = "shared", issue = "27730")]
// impl<T: ?Sized, U: ?Sized> CoerceUnsized<Shared<U>> for Shared<T> where T: Unsize<U> { }

// #[unstable(feature = "shared", issue = "27730")]
impl<T: ?Sized> Deref for Shared<T> {
    type Target = *mut T;

    #[inline]
    fn deref(&self) -> &*mut T {
        unsafe { mem::transmute(&self.pointer) }
    }
}

// #[unstable(feature = "shared", issue = "27730")]
impl<T> fmt::Pointer for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.pointer, f)
    }
}
