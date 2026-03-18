//! # SinStr - Compact String with Small String Optimization
//!
//! A compact string type that fits in a single `usize` using Small String Optimization (SSO).
//!
//! ## Key Properties
//!
//! - **Size**: Exactly `size_of::<usize>()` bytes (8 bytes on 64-bit)
//! - **Inline capacity**: Platform-dependent (7 bytes on 64-bit, 3 on 32-bit, 1 on 16-bit)
//! - **Zero-cost**: No heap allocation for short strings
//! - **Niche-optimized**: `Option<SinStr>` is the same size as `SinStr`
//!
//! ## Memory Layout
//!
//! Uses the low bits of a `usize` to store the discriminant. Heap pointers are aligned to
//! `align_of::<usize>()` (8 on 64-bit), so low bits are always zero for valid heap pointers.
//!
//! ### Inline (length <= NICHE_MAX_INT)
//! - Discriminant byte: length (1..=NICHE_MAX_INT)
//! - Remaining bytes: string data
//!
//! ### Heap (length > NICHE_MAX_INT)
//! - Full pointer stored
//! - Length stored at `ptr - 1` (before string data)
//! - High byte acts as heap discriminant (non-zero)
//!
//! ## Platform Differences
//!
//! | Platform | NICHE_BITS | Max Inline |
//! |----------|------------|------------|
//! | 64-bit   | 3          | 7 bytes    |
//! | 32-bit   | 2          | 3 bytes    |
//! | 16-bit   | 1          | 1 byte     |
//!
//! ## Example
//!
//! ```
//! use sinstr::SinStr;
//!
//! let small = SinStr::new("hi");  // Inline on 64-bit
//! let large = SinStr::new("hello world"); // Heap
//!
//! assert!(small.is_inlined());
//! assert!(large.is_heap());
//! ```

use core::str;
use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error},
    mem::{MaybeUninit, size_of, transmute},
    num::{NonZeroU8, NonZeroUsize},
    ptr::NonNull,
};

mod discriminant;
pub use discriminant::DiscriminantValues;

use crate::discriminant::NICHE_MAX_INT;

#[repr(C)]
struct HeapRepr(NonZeroUsize);

impl HeapRepr {
    fn as_ptr(&self) -> *const usize {
        core::ptr::with_exposed_provenance::<usize>(self.0.get())
    }

    fn as_ptr_mut(&self) -> *mut usize {
        core::ptr::with_exposed_provenance_mut::<usize>(self.0.get())
    }

    fn len(&self) -> usize {
        unsafe { self.as_ptr().read() }
    }

    fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let len = self.len();
        unsafe {
            core::ptr::slice_from_raw_parts(ptr.add(1).cast::<u8>(), len)
                .as_ref()
                .unwrap_unchecked()
        }
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = self.as_ptr_mut();
        let len = self.len();
        unsafe {
            core::ptr::slice_from_raw_parts_mut(ptr.add(1).cast::<u8>(), len)
                .as_mut()
                .unwrap_unchecked()
        }
    }

    fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    fn as_str_mut(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
    }
}

#[repr(C)]
struct InlinedRepr {
    #[cfg(target_endian = "big")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    len: NonZeroU8,
    #[cfg(target_endian = "little")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl InlinedRepr {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            (self.data.get_unchecked(..self.len.get() as usize) as *const [MaybeUninit<u8>]
                as *const [u8])
                .as_ref()
                .unwrap_unchecked()
        }
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            (self.data.get_unchecked_mut(..self.len.get() as usize) as *mut [MaybeUninit<u8>]
                as *mut [u8])
                .as_mut()
                .unwrap_unchecked()
        }
    }

    fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    fn as_str_mut(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
    }
}

#[repr(C)]
struct Repr {
    _align: [usize; 0], // Zero-sized, forces usize alignment
    #[cfg(target_endian = "big")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    disc: DiscriminantValues,
    #[cfg(target_endian = "little")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl Repr {
    pub fn is_inlined(&self) -> bool {
        let len = self.disc as usize;
        len > 0 && len <= NICHE_MAX_INT
    }

    pub fn is_heap(&self) -> bool {
        !self.is_inlined()
    }

    pub fn len(&self) -> usize {
        if self.is_inlined() {
            self.disc as usize
        } else {
            unsafe { self.get_heap() }.len()
        }
    }

    unsafe fn get_heap(&self) -> &HeapRepr {
        const _: () = assert!(size_of::<Repr>() == size_of::<usize>());
        unsafe {
            // SAFETY: We are picking up a previously exposed provenance.
            // Repr is the same size as usize and we have confirmed we are dealing with a pointer.
            // The feature gate in the struct definition ensure that endiannes is accounted for.
            //
            // The transmute is safe as we have confirmed they are the same size and have the same
            // alignment.
            transmute::<&Repr, &HeapRepr>(self)
        }
    }

    unsafe fn get_heap_mut(&mut self) -> &mut HeapRepr {
        const _: () = assert!(size_of::<Repr>() == size_of::<usize>());
        unsafe {
            // SAFETY: We are picking up a previously exposed provenance.
            // Repr is the same size as usize and we have confirmed we are dealing with a pointer.
            // The feature gate in the struct definition ensure that endiannes is accounted for.
            //
            // The transmute is safe as we have confirmed they are the same size and have the same
            // alignment.
            transmute::<&mut Repr, &mut HeapRepr>(self)
        }
    }

    fn get_inlined(&self) -> &InlinedRepr {
        unsafe { transmute(self) }
    }

    fn get_inlined_mut(&mut self) -> &mut InlinedRepr {
        unsafe { transmute(self) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        if self.is_inlined() {
            self.get_inlined().as_bytes()
        } else {
            unsafe { self.get_heap() }.as_bytes()
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        if self.is_inlined() {
            self.get_inlined_mut().as_bytes_mut()
        } else {
            unsafe { self.get_heap_mut() }.as_bytes_mut()
        }
    }

    pub fn as_str(&self) -> &str {
        if self.is_inlined() {
            self.get_inlined().as_str()
        } else {
            unsafe { self.get_heap() }.as_str()
        }
    }

    pub fn as_str_mut(&mut self) -> &mut str {
        if self.is_inlined() {
            self.get_inlined_mut().as_str_mut()
        } else {
            unsafe { self.get_heap_mut() }.as_str_mut()
        }
    }
}

impl Drop for Repr {
    fn drop(&mut self) {
        if self.is_heap() {
            let heap = unsafe { self.get_heap_mut() };
            let ptr = heap.as_ptr_mut();
            let len = heap.len();
            unsafe {
                let layout = Layout::from_size_align(size_of::<usize>() + len, align_of::<usize>())
                    .unwrap_unchecked();
                dealloc(ptr as _, layout)
            };
        }
    }
}

// Ensure Repr and Option<Repr> is NPO
const _: () = assert!(size_of::<Repr>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<Repr>>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<Repr>>() >= align_of::<usize>());

#[repr(transparent)]
pub struct SinStr(Option<Repr>);

impl SinStr {
    pub fn new(s: &str) -> Self {
        let len = s.len();
        if len == 0 {
            return Self(None);
        }

        if NICHE_MAX_INT >= len {
            unsafe { Self::new_inline(s) }
        } else {
            unsafe { Self::new_heap(s) }
        }
    }

    /// Creates a new `SinStr` that stores data in the `SinStr` directly.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be less than or equal to [`NICHE_MAX_INT`].
    const unsafe fn new_inline(s: &str) -> Self {
        let len = s.len();
        debug_assert!(len <= NICHE_MAX_INT);
        let mut buf = [MaybeUninit::uninit(); size_of::<NonZeroUsize>() - 1];
        let mut i = 0;
        while i < len {
            buf[i] = MaybeUninit::new(s.as_bytes()[i]);
            i += 1;
        }

        // SAFETY: len is less than NICHE_MAX_INT and all versions of DiscriminantValues have variants with that value.
        unsafe {
            Self(Some(Repr {
                _align: [],
                disc: transmute::<u8, discriminant::DiscriminantValues>(len as u8),
                data_or_partial_ptr: buf,
            }))
        }
    }

    /// Creates a new `SinStr` that stores data on the heap.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be greater than [`NICHE_MAX_INT`].
    unsafe fn new_heap(s: &str) -> Self {
        let len = s.len();
        debug_assert!(len > NICHE_MAX_INT);
        let total_size = size_of::<usize>()
            .checked_add(len)
            .expect("string too large");
        let layout = Layout::from_size_align(total_size, align_of::<usize>()).unwrap();

        // SAFETY: layout size > 0 because len > NICHE_MAX_INT > 0
        let Some(ptr) = NonNull::new(unsafe { alloc(layout) }) else {
            handle_alloc_error(layout)
        };

        // SAFETY: We allocated for a usize + len and the pointer is properly aligned.
        unsafe {
            ptr.cast::<usize>().write(len);
            ptr.add(size_of::<usize>())
                .cast::<MaybeUninit<u8>>()
                .as_ptr()
                .copy_from_nonoverlapping(s.as_bytes().as_ptr() as _, len);
        }

        // SAFETY: Repr is #[repr(C)] and exactly size_of::<usize>() bytes.
        // The discriminant byte will be the high byte of the pointer.
        // Heap pointers on most architectures have high byte > NICHE_MAX_INT,
        // ensuring is_heap() returns true.
        unsafe {
            Self(Some(transmute::<usize, Repr>(
                ptr.as_ptr().expose_provenance(),
            )))
        }
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            None => 0,
            Some(repr) => repr.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            None => "",
            Some(repr) => repr.as_str(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            None => b"",
            Some(repr) => repr.as_bytes(),
        }
    }

    pub fn is_inlined(&self) -> bool {
        match &self.0 {
            None => false,
            Some(repr) => repr.is_inlined(),
        }
    }

    pub fn is_heap(&self) -> bool {
        match &self.0 {
            None => false,
            Some(repr) => repr.is_heap(),
        }
    }
}

// Ensure SinStr and Option<SinStr> is NPO
const _: () = assert!(size_of::<SinStr>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<SinStr>>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<SinStr>>() >= align_of::<usize>());

#[cfg(test)]
mod tests {
    use crate::SinStr;

    #[allow(unused)]
    enum MyEnum {
        A(SinStr),
        B(u32),
    }

    // Ensure that the compiler is using the niches for enums.
    // This isn't a safety requirement or guarantee we provide so putting this in the tests is fine.
    const _: () = assert!(size_of::<MyEnum>() == size_of::<SinStr>());

    #[test]
    fn test_empty_string() {
        let s = SinStr::new("");
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.as_str(), "");
        assert_eq!(s.as_bytes(), b"");
        assert!(!s.is_inlined());
        assert!(!s.is_heap());
    }

    #[test]
    fn test_inline_string() {
        use crate::discriminant::NICHE_MAX_INT;

        // Length 1 is always inline on all platforms
        let s = SinStr::new("a");
        assert!(!s.is_empty());
        assert_eq!(s.len(), 1);
        assert_eq!(s.as_str(), "a");
        assert_eq!(s.as_bytes(), b"a");
        assert!(s.is_inlined());
        assert!(!s.is_heap());

        // Length 2 is inline on 32-bit and 64-bit, heap on 16-bit
        if NICHE_MAX_INT >= 2 {
            let s = SinStr::new("ab");
            assert_eq!(s.len(), 2);
            assert_eq!(s.as_str(), "ab");
            assert!(s.is_inlined());
        }

        // Length 3 is inline on 64-bit only
        if NICHE_MAX_INT >= 3 {
            let s = SinStr::new("abc");
            assert_eq!(s.len(), 3);
            assert_eq!(s.as_str(), "abc");
            assert_eq!(s.as_bytes(), b"abc");
            assert!(s.is_inlined());
        }

        // Max inline length for this platform
        let max_inline = "x".repeat(NICHE_MAX_INT);
        let s = SinStr::new(&max_inline);
        assert_eq!(s.len(), NICHE_MAX_INT);
        assert_eq!(s.as_str(), max_inline);
        assert_eq!(s.as_bytes(), max_inline.as_bytes());
        assert!(s.is_inlined());
        assert!(!s.is_heap());
    }

    #[test]
    fn test_heap_string() {
        use crate::discriminant::NICHE_MAX_INT;

        // On 16-bit: NICHE_MAX_INT = 1, so length 2 is heap
        // On 32-bit: NICHE_MAX_INT = 3, so length 4 is heap
        // On 64-bit: NICHE_MAX_INT = 7, so length 8 is heap
        let first_heap = "x".repeat(NICHE_MAX_INT + 1);
        let s = SinStr::new(&first_heap);
        assert!(!s.is_empty());
        assert_eq!(s.len(), NICHE_MAX_INT + 1);
        assert_eq!(s.as_str(), first_heap);
        assert_eq!(s.as_bytes(), first_heap.as_bytes());
        assert!(!s.is_inlined());
        assert!(s.is_heap());

        // Test length 2 on 16-bit (which is heap)
        if NICHE_MAX_INT < 2 {
            let s = SinStr::new("ab");
            assert_eq!(s.len(), 2);
            assert_eq!(s.as_str(), "ab");
            assert!(!s.is_inlined());
            assert!(s.is_heap());
        }

        // Test length 3 on 16-bit and 32-bit (which is heap on those platforms)
        if NICHE_MAX_INT < 3 {
            let s = SinStr::new("abc");
            assert_eq!(s.len(), 3);
            assert_eq!(s.as_str(), "abc");
            assert!(!s.is_inlined());
            assert!(s.is_heap());
        }

        // Large heap allocation (works on all platforms)
        let large = "x".repeat(100);
        let s = SinStr::new(&large);
        assert_eq!(s.len(), 100);
        assert_eq!(s.as_str(), large);
        assert_eq!(s.as_bytes(), large.as_bytes());
        assert!(!s.is_inlined());
        assert!(s.is_heap());
    }
}
