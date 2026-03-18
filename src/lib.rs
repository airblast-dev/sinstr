#![no_std]
extern crate alloc;
use alloc::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use core::{
    mem::{MaybeUninit, size_of, transmute},
    num::{NonZeroU8, NonZeroUsize},
    ptr::NonNull,
    str,
};

mod discriminant;
pub use discriminant::DiscriminantValues;

use crate::discriminant::NICHE_MAX_INT;

#[repr(C)]
pub struct HeapRepr(NonZeroUsize);

impl HeapRepr {
    pub fn as_ptr(&self) -> NonNull<NonZeroUsize> {
        NonNull::with_exposed_provenance(self.0)
    }

    pub fn as_ptr_mut(&mut self) -> NonNull<NonZeroUsize> {
        NonNull::with_exposed_provenance(self.0)
    }

    /// Returns the length of the stored string.
    ///
    /// Returns a [`NonZeroUsize`] as [`HeapRepr`] is always greater than [`NICHE_MAX_INT`].
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> NonZeroUsize {
        unsafe { self.as_ptr().read() }
    }

    /// Returns the string as a slice of bytes.
    fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let len = self.len();
        unsafe { NonNull::slice_from_raw_parts(ptr.add(1).cast::<u8>(), len.get()).as_ref() }
    }

    /// Returns the string as a mutable slice of bytes.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = self.as_ptr_mut();
        let len = self.len();
        unsafe { NonNull::slice_from_raw_parts(ptr.add(1).cast::<u8>(), len.get()).as_mut() }
    }

    /// Returns the string as a `&str`.
    fn as_str(&self) -> &str {
        // SAFETY: The bytes were copied from a valid &str during construction
        // and haven't been mutated, so they remain valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Returns the string as a `&mut str`.
    fn as_str_mut(&mut self) -> &mut str {
        // SAFETY: The bytes were copied from a valid &str during construction.
        // The caller of as_str_mut() must preserve UTF-8 validity.
        unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
    }
}

#[repr(C)]
pub struct InlinedRepr {
    #[cfg(target_endian = "big")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    len: NonZeroU8,
    #[cfg(target_endian = "little")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl InlinedRepr {
    /// Returns the string as a slice of bytes.
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            (self.data.get_unchecked(..self.len.get() as usize) as *const [MaybeUninit<u8>]
                as *const [u8])
                .as_ref()
                .unwrap_unchecked()
        }
    }

    /// Returns the string as a mutable slice of bytes.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            (self.data.get_unchecked_mut(..self.len.get() as usize) as *mut [MaybeUninit<u8>]
                as *mut [u8])
                .as_mut()
                .unwrap_unchecked()
        }
    }

    /// Returns the string as a `&str`.
    fn as_str(&self) -> &str {
        // SAFETY: The bytes were copied from a valid &str during construction
        // and haven't been mutated, so they remain valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Returns the string as a `&mut str`.
    fn as_str_mut(&mut self) -> &mut str {
        // SAFETY: The bytes were copied from a valid &str during construction.
        // The caller of as_str_mut() must preserve UTF-8 validity.
        unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
    }
}

#[repr(C)]
pub struct InnerSinStr {
    _align: [usize; 0], // Zero-sized, forces usize alignment
    #[cfg(target_endian = "big")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    disc: DiscriminantValues,
    #[cfg(target_endian = "little")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl InnerSinStr {
    /// Create a new [`InnerSinStr`]
    ///
    /// Returns [`None`] if the string is empty.
    pub fn new(s: &str) -> Option<Self> {
        let len = s.len();
        if len == 0 {
            return None;
        }

        Some(if NICHE_MAX_INT >= len {
            // SAFETY: we have ensured `s` fits in an inline string
            unsafe { Self::new_inline(s) }
        } else {
            // SAFETY: we have ensured `s` does not fit in an inline string
            unsafe { Self::new_heap(s) }
        })
    }

    /// Creates a new `SinStr` that stores data in the `SinStr` directly.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be less than or equal to [`NICHE_MAX_INT`].
    pub const unsafe fn new_inline(s: &str) -> Self {
        let len = s.len();
        debug_assert!(len <= NICHE_MAX_INT && len > 0);
        let mut buf = [MaybeUninit::uninit(); size_of::<NonZeroUsize>() - 1];
        let mut i = 0;
        while i < len {
            buf[i] = MaybeUninit::new(s.as_bytes()[i]);
            i += 1;
        }

        // SAFETY: len is less than or equal to NICHE_MAX_INT and all versions of DiscriminantValues have variants with that value.
        unsafe {
            InnerSinStr {
                _align: [],
                disc: transmute::<u8, discriminant::DiscriminantValues>(len as u8),
                data_or_partial_ptr: buf,
            }
        }
    }

    /// Creates a new `SinStr` that stores data on the heap.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be greater than [`NICHE_MAX_INT`].
    pub unsafe fn new_heap(s: &str) -> Self {
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
        unsafe { transmute::<usize, InnerSinStr>(ptr.as_ptr().expose_provenance()) }
    }

    pub const fn is_inlined(&self) -> bool {
        let len = self.disc as usize;
        len > 0 && len <= NICHE_MAX_INT
    }

    pub fn is_heap(&self) -> bool {
        !self.is_inlined()
    }

    pub fn len(&self) -> NonZeroUsize {
        if self.is_inlined() {
            // SAFETY: is_inlined() guarantees the discriminant represents a valid
            // inline length in range 1..=NICHE_MAX_INT, which is always non-zero.
            unsafe { NonZeroUsize::new_unchecked(self.disc as usize) }
        } else {
            unsafe { self.get_heap() }.len()
        }
    }

    /// Get the heap repr for the [`InnerSinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is heap allocated.
    pub unsafe fn get_heap(&self) -> &HeapRepr {
        const _: () = assert!(size_of::<InnerSinStr>() == size_of::<usize>());
        const _: () = assert!(align_of::<InnerSinStr>() == align_of::<usize>());
        unsafe {
            // SAFETY: We are picking up a previously exposed provenance.
            // Repr is the same size as usize and we have confirmed we are dealing with a pointer.
            // The feature gate in the struct definition ensure that endiannes is accounted for.
            //
            // The transmute is safe as we have confirmed they are the same size and have the same
            // alignment.
            transmute::<&InnerSinStr, &HeapRepr>(self)
        }
    }

    /// Get the heap repr for the [`InnerSinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is heap allocated.
    pub unsafe fn get_heap_mut(&mut self) -> &mut HeapRepr {
        const _: () = assert!(size_of::<InnerSinStr>() == size_of::<usize>());
        unsafe {
            // SAFETY: We are picking up a previously exposed provenance.
            // Repr is the same size as usize and we have confirmed we are dealing with a pointer.
            // The feature gate in the struct definition ensure that endiannes is accounted for.
            //
            // The transmute is safe as we have confirmed they are the same size and have the same
            // alignment.
            transmute::<&mut InnerSinStr, &mut HeapRepr>(self)
        }
    }

    /// Get the inline repr for the [`InnerSinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is inlined.
    pub const unsafe fn get_inlined(&self) -> &InlinedRepr {
        // SAFETY: Self and InlinedRepr have the same size and alignment.
        unsafe { transmute(self) }
    }

    /// Get the inline repr for the [`InnerSinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is inlined.
    pub const unsafe fn get_inlined_mut(&mut self) -> &mut InlinedRepr {
        unsafe { transmute(self) }
    }

    /// Returns the string as a slice of bytes.
    pub fn as_bytes(&self) -> &[u8] {
        if self.is_inlined() {
            // SAFETY: just checked that the string is inlined
            unsafe { self.get_inlined() }.as_bytes()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap() }.as_bytes()
        }
    }

    /// Returns the string as a mutable slice of bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        if self.is_inlined() {
            // SAFETY: just checked that the string is inlined
            unsafe { self.get_inlined_mut() }.as_bytes_mut()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap_mut() }.as_bytes_mut()
        }
    }

    /// Returns the string as a `&str`.
    pub fn as_str(&self) -> &str {
        if self.is_inlined() {
            // SAFETY: just checked that the string is inlined
            unsafe { self.get_inlined() }.as_str()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap() }.as_str()
        }
    }

    /// Returns the string as a `&mut str`.
    pub fn as_str_mut(&mut self) -> &mut str {
        if self.is_inlined() {
            // SAFETY: just checked that the string is inlined
            unsafe { self.get_inlined_mut() }.as_str_mut()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap_mut() }.as_str_mut()
        }
    }
}

impl Drop for InnerSinStr {
    fn drop(&mut self) {
        if self.is_heap() {
            // SAFETY: just checked that the string is on the heap
            let heap = unsafe { self.get_heap_mut() };
            let ptr = heap.as_ptr_mut();
            let len = heap.len();
            unsafe {
                let layout =
                    Layout::from_size_align(size_of::<usize>() + len.get(), align_of::<usize>())
                        .unwrap_unchecked();
                dealloc(ptr.cast::<u8>().as_ptr(), layout)
            };
        }
    }
}

// Ensure Repr and Option<Repr> is NPO
const _: () = assert!(size_of::<InnerSinStr>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<InnerSinStr>>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<InnerSinStr>>() >= align_of::<usize>());

#[repr(transparent)]
pub struct SinStr(Option<InnerSinStr>);

impl SinStr {
    pub fn new(s: &str) -> Self {
        Self(InnerSinStr::new(s))
    }

    /// Creates a new `SinStr` that stores data in the `SinStr` directly.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be less than or equal to [`NICHE_MAX_INT`] but
    /// greater than `0`.
    pub const unsafe fn new_inline(s: &str) -> Self {
        Self(unsafe { Some(InnerSinStr::new_inline(s)) })
    }

    /// Creates a new `SinStr` that stores data on the heap.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be greater than [`NICHE_MAX_INT`].
    pub unsafe fn new_heap(s: &str) -> Self {
        Self(unsafe { Some(InnerSinStr::new_heap(s)) })
    }

    /// Returns the length of the string.
    pub fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |r| r.len().get())
    }

    /// Returns `true` if the string is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Returns the string as a `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_ref().map_or("", InnerSinStr::as_str)
    }

    /// Returns the string as a `&mut str`.
    pub fn as_str_mut(&mut self) -> &mut str {
        match &mut self.0 {
            Some(r) => r.as_str_mut(),
            r @ None => unsafe {
                str::from_utf8_unchecked_mut(core::slice::from_raw_parts_mut(
                    r as *mut Option<InnerSinStr> as *mut u8,
                    0,
                ))
            },
        }
    }

    /// Returns the string as a slice of bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref().map_or(b"", InnerSinStr::as_bytes)
    }

    /// Returns the string as a mutable slice of bytes.
    ///
    /// # Safety
    ///
    /// The bytes must be valid UTF-8.
    pub unsafe fn as_byte_mut(&mut self) -> &mut [u8] {
        match &mut self.0 {
            Some(r) => r.as_bytes_mut(),
            r @ None => unsafe {
                core::slice::from_raw_parts_mut(r as *mut Option<InnerSinStr> as *mut u8, 0)
            },
        }
    }

    pub fn is_inlined(&self) -> bool {
        // Implementation detail but we consider an empty string inlined
        self.0.as_ref().is_none_or(InnerSinStr::is_inlined)
    }

    pub fn is_heap(&self) -> bool {
        self.0.as_ref().is_some_and(InnerSinStr::is_heap)
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
        assert!(s.is_inlined());
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
