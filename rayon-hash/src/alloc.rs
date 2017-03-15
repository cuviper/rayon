//! Stripped down copy of the standard alloc
//!
//! This cheats by using internal allocator names.

pub mod heap {
    use std::isize;

    #[allow(improper_ctypes)]
    extern "C" {
        fn __rust_allocate(size: usize, align: usize) -> *mut u8;
        fn __rust_deallocate(ptr: *mut u8, old_size: usize, align: usize);
    }

    #[inline(always)]
    fn check_size_and_alignment(size: usize, align: usize) {
        debug_assert!(size != 0);
        debug_assert!(size <= isize::MAX as usize,
                      "Tried to allocate too much: {} bytes",
                      size);
        debug_assert!(usize::is_power_of_two(align),
                      "Invalid alignment of allocation: {}",
                      align);
    }

    // FIXME: #13996: mark the `allocate` and `reallocate` return value as `noalias`

    /// Return a pointer to `size` bytes of memory aligned to `align`.
    ///
    /// On failure, return a null pointer.
    ///
    /// Behavior is undefined if the requested size is 0 or the alignment is not a
    /// power of 2. The alignment must be no larger than the largest supported page
    /// size on the platform.
    #[inline]
    pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
        check_size_and_alignment(size, align);
        __rust_allocate(size, align)
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// The `ptr` parameter must not be null.
    ///
    /// The `old_size` and `align` parameters are the parameters that were used to
    /// create the allocation referenced by `ptr`. The `old_size` parameter may be
    /// any value in range_inclusive(requested_size, usable_size).
    #[inline]
    pub unsafe fn deallocate(ptr: *mut u8, old_size: usize, align: usize) {
        __rust_deallocate(ptr, old_size, align)
    }

    /// An arbitrary non-null address to represent zero-size allocations.
    ///
    /// This preserves the non-null invariant for types like `Box<T>`. The address
    /// may overlap with non-zero-size memory allocations.
    pub const EMPTY: *mut () = 0x1 as *mut ();
}

pub fn oom() -> ! {
    panic!("out of memory!");
}
