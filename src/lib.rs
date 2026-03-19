#![no_std]
extern crate alloc;
use alloc::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use core::{
    hint::assert_unchecked,
    mem::{MaybeUninit, size_of, transmute},
    num::{NonZeroU8, NonZeroUsize},
    ptr::{self, NonNull},
    str,
};

mod discriminant;
pub use discriminant::{DiscriminantValues, NICHE_MAX_INT};

#[inline]
#[cold]
const fn cold() {}

#[inline]
const fn likely(b: bool) -> bool {
    if !b {
        cold()
    }
    b
}

#[inline]
const fn unlikely(b: bool) -> bool {
    if b {
        cold()
    }
    b
}

#[repr(C)]
pub struct HeapRepr(NonZeroUsize);

impl HeapRepr {
    #[inline]
    pub fn as_ptr(&self) -> NonNull<NonZeroUsize> {
        NonNull::with_exposed_provenance(self.0)
    }

    #[inline]
    pub fn as_ptr_mut(&mut self) -> NonNull<NonZeroUsize> {
        NonNull::with_exposed_provenance(self.0)
    }

    /// Returns the length of the stored string.
    ///
    /// Returns a [`NonZeroUsize`] as [`HeapRepr`] is always greater than [`NICHE_MAX_INT`].
    #[allow(clippy::len_without_is_empty)]
    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        // SAFETY: pointer is always non null and properly aligned with enough provenance to read a usize
        unsafe { self.as_ptr().read() }
    }

    /// Returns the string as a slice of bytes.
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let len = self.len();
        // SAFETY: pointer is always non null and properly aligned with enough provenance to read a usize + len bytes
        unsafe { NonNull::slice_from_raw_parts(ptr.add(1).cast::<u8>(), len.get()).as_ref() }
    }

    /// Returns the string as a mutable slice of bytes.
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = self.as_ptr_mut();
        let len = self.len();
        unsafe { NonNull::slice_from_raw_parts(ptr.add(1).cast::<u8>(), len.get()).as_mut() }
    }

    /// Returns the string as a `&str`.
    #[inline]
    fn as_str(&self) -> &str {
        // SAFETY: The bytes were copied from a valid &str during construction
        // and haven't been mutated, so they remain valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Returns the string as a `&mut str`.
    #[inline]
    fn as_str_mut(&mut self) -> &mut str {
        // SAFETY: The bytes were copied from a valid &str during construction.
        // The caller of as_str_mut() must preserve UTF-8 validity.
        unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
    }
}

#[repr(C)]
pub struct InlinedRepr {
    _align: [usize; 0],
    #[cfg(target_endian = "little")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    len: NonZeroU8,
    #[cfg(target_endian = "big")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl InlinedRepr {
    /// Returns the string as a slice of bytes.
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            (self.data.get_unchecked(..self.len.get() as usize) as *const [MaybeUninit<u8>]
                as *const [u8])
                .as_ref()
                .unwrap_unchecked()
        }
    }

    /// Returns the string as a mutable slice of bytes.
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            (self.data.get_unchecked_mut(..self.len.get() as usize) as *mut [MaybeUninit<u8>]
                as *mut [u8])
                .as_mut()
                .unwrap_unchecked()
        }
    }

    /// Returns the string as a `&str`.
    #[inline(always)]
    fn as_str(&self) -> &str {
        // SAFETY: The bytes were copied from a valid &str during construction
        // and haven't been mutated, so they remain valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Returns the string as a `&mut str`.
    #[inline]
    fn as_str_mut(&mut self) -> &mut str {
        // SAFETY: The bytes were copied from a valid &str during construction.
        // The caller of as_str_mut() must preserve UTF-8 validity.
        unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
    }
}

#[repr(C)]
pub struct NonEmptySinStr {
    _align: [usize; 0], // Zero-sized, forces usize alignment
    #[cfg(target_endian = "little")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    disc: DiscriminantValues,
    #[cfg(target_endian = "big")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl NonEmptySinStr {
    /// Create a new [`NonEmptySinStr`]
    ///
    /// Returns [`None`] if the string is empty.
    #[inline]
    pub fn new(s: &str) -> Option<Self> {
        let len = s.len();
        if len == 0 {
            return None;
        }

        Some(if likely(NICHE_MAX_INT >= len) {
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
    #[inline]
    pub const unsafe fn new_inline(s: &str) -> Self {
        let len = s.len();
        debug_assert!(len <= NICHE_MAX_INT && len > 0);
        unsafe { assert_unchecked(len > 0 && len <= NICHE_MAX_INT) };
        let mut buf = [MaybeUninit::uninit(); size_of::<NonZeroUsize>() - 1];

        // Use copy_nonoverlapping for better performance than byte-by-byte copy
        unsafe {
            ptr::copy_nonoverlapping(s.as_ptr(), buf.as_mut_ptr().cast::<u8>(), len);
        }

        // SAFETY: len is less than or equal to NICHE_MAX_INT and all versions of DiscriminantValues have variants with that value.
        unsafe {
            NonEmptySinStr {
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
        unsafe { assert_unchecked(len > NICHE_MAX_INT) };
        let total_size = size_of::<usize>() + len;
        // SAFETY: align_of::<usize>() is always valid (power of 2) and total_size > 0 because len > NICHE_MAX_INT > 0
        let layout = unsafe { Layout::from_size_align_unchecked(total_size, align_of::<usize>()) };

        // SAFETY: layout size > 0 because len > NICHE_MAX_INT > 0
        let Some(ptr) = NonNull::new(unsafe { alloc(layout) }) else {
            handle_alloc_error(layout)
        };

        // SAFETY: We allocated for a usize + len and the pointer is properly aligned.
        unsafe {
            ptr.cast::<usize>().write(len);
            ptr.add(size_of::<usize>())
                .cast::<u8>()
                .copy_from_nonoverlapping(NonNull::new_unchecked(s.as_ptr() as *mut u8), len);
            // SAFETY: Repr is #[repr(C)] and exactly size_of::<usize>() bytes.
            transmute::<usize, NonEmptySinStr>(ptr.expose_provenance().get())
        }
    }

    #[inline(always)]
    pub const fn is_inlined(&self) -> bool {
        // If the discriminant is less than NICHE_MAX_INT but greater than 0
        // Then it means the pointer isn't properly aligned making it an inlined string.
        //
        // We are using the heap pointers alignment requirements as the niche to detect if we are inlined.
        // If on the heap the LSB `NICHE_BITS` are always zero.
        //
        // This is also why we can't store empty strings in the inner repr as the length value is all zero bits.
        let len = self.disc as usize;
        // No branching since the sub just wraps
        likely((len.wrapping_sub(1)) < NICHE_MAX_INT)
    }

    #[inline]
    pub const fn is_heap(&self) -> bool {
        unlikely(!self.is_inlined())
    }

    #[inline]
    pub fn len(&self) -> NonZeroUsize {
        if self.is_inlined() {
            // SAFETY: is_inlined() guarantees the discriminant represents a valid
            // inline length in range 1..=NICHE_MAX_INT, which is always non-zero.
            unsafe { NonZeroUsize::new_unchecked(self.disc as usize) }
        } else {
            unsafe { self.get_heap() }.len()
        }
    }

    /// Get the heap repr for the [`NonEmptySinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is heap allocated.
    pub unsafe fn get_heap(&self) -> &HeapRepr {
        const _: () = assert!(size_of::<NonEmptySinStr>() == size_of::<usize>());
        const _: () = assert!(align_of::<NonEmptySinStr>() == align_of::<usize>());
        unsafe {
            (self as *const NonEmptySinStr as *const HeapRepr)
                .as_ref()
                .unwrap_unchecked()
        }
    }

    /// Get the heap repr for the [`NonEmptySinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is heap allocated.
    pub unsafe fn get_heap_mut(&mut self) -> &mut HeapRepr {
        const _: () = assert!(size_of::<NonEmptySinStr>() == size_of::<usize>());
        unsafe {
            (self as *mut NonEmptySinStr as *mut HeapRepr)
                .as_mut()
                .unwrap_unchecked()
        }
    }

    /// Get the inline repr for the [`NonEmptySinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is inlined.
    #[inline(always)]
    pub const unsafe fn get_inlined(&self) -> &InlinedRepr {
        // SAFETY: Self and InlinedRepr have the same size and alignment.
        unsafe { transmute(self) }
    }

    /// Get the inline repr for the [`NonEmptySinStr`].
    ///
    /// # Safety
    ///
    /// Caller must ensure that the string is inlined.
    #[inline]
    pub unsafe fn get_inlined_mut(&mut self) -> &mut InlinedRepr {
        unsafe {
            (self as *mut NonEmptySinStr as *mut InlinedRepr)
                .as_mut()
                .unwrap_unchecked()
        }
    }

    /// Returns the string as a slice of bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: just checked that the string is inlined
        if likely(self.is_inlined()) {
            unsafe { self.get_inlined() }.as_bytes()
        } else {
            unsafe { self.get_heap() }.as_bytes()
        }
    }

    /// Returns the string as a mutable slice of bytes.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        if likely(self.is_inlined()) {
            unsafe { self.get_inlined_mut() }.as_bytes_mut()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap_mut() }.as_bytes_mut()
        }
    }

    /// Returns the string as a `&str`.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        // SAFETY: just checked that the string is inlined
        if likely(self.is_inlined()) {
            unsafe { self.get_inlined() }.as_str()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap() }.as_str()
        }
    }

    /// Returns the string as a `&mut str`.
    #[inline]
    pub fn as_str_mut(&mut self) -> &mut str {
        // SAFETY: just checked that the string is inlined
        if likely(self.is_inlined()) {
            unsafe { self.get_inlined_mut() }.as_str_mut()
        } else {
            // SAFETY: just checked that the string is not inlined
            unsafe { self.get_heap_mut() }.as_str_mut()
        }
    }
}

impl Drop for NonEmptySinStr {
    #[inline]
    fn drop(&mut self) {
        if self.is_heap() {
            // SAFETY: just checked that the string is on the heap
            unsafe { self.drop_heap() };
        }
    }
}

impl NonEmptySinStr {
    #[cold]
    unsafe fn drop_heap(&mut self) {
        unsafe {
            let heap = self.get_heap_mut();
            let ptr = heap.as_ptr_mut();
            let len = heap.len();
            let layout = Layout::from_size_align_unchecked(
                size_of::<usize>().unchecked_add(len.get()),
                align_of::<usize>(),
            );
            dealloc(ptr.cast::<u8>().as_ptr(), layout)
        }
    }
}

// Ensure Repr and Option<Repr> is NPO
const _: () = assert!(size_of::<NonEmptySinStr>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<NonEmptySinStr>>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<NonEmptySinStr>>() >= align_of::<usize>());

#[repr(transparent)]
pub struct SinStr(Option<NonEmptySinStr>);

impl SinStr {
    #[inline]
    pub fn new(s: &str) -> Self {
        Self(NonEmptySinStr::new(s))
    }

    /// Creates a new `SinStr` that stores data in the `SinStr` directly.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be less than or equal to [`NICHE_MAX_INT`] but
    /// greater than `0`.
    #[inline]
    pub const unsafe fn new_inline(s: &str) -> Self {
        Self(unsafe { Some(NonEmptySinStr::new_inline(s)) })
    }

    /// Creates a new `SinStr` that stores data on the heap.
    ///
    /// # Safety
    ///
    /// The length of the provided string must be greater than [`NICHE_MAX_INT`].
    #[inline]
    pub unsafe fn new_heap(s: &str) -> Self {
        Self(unsafe { Some(NonEmptySinStr::new_heap(s)) })
    }

    /// Returns the length of the string.
    #[inline(always)]
    pub fn len(&self) -> usize {
        match &self.0 {
            Some(r) => r.len().get(),
            None => 0,
        }
    }

    /// Returns `true` if the string is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Returns the string as a `&str`.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Some(r) => r.as_str(),
            None => "",
        }
    }

    /// Returns the string as a `&mut str`.
    #[inline]
    pub fn as_str_mut(&mut self) -> &mut str {
        match &mut self.0 {
            Some(r) => r.as_str_mut(),
            r @ None => unsafe {
                str::from_utf8_unchecked_mut(core::slice::from_raw_parts_mut(
                    r as *mut Option<NonEmptySinStr> as *mut u8,
                    0,
                ))
            },
        }
    }

    /// Returns the string as a slice of bytes.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            Some(r) => r.as_bytes(),
            None => b"",
        }
    }

    /// Returns the string as a mutable slice of bytes.
    ///
    /// # Safety
    ///
    /// After mutation, the bytes must remain valid UTF-8.
    #[inline]
    pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        match &mut self.0 {
            Some(r) => r.as_bytes_mut(),
            r @ None => unsafe {
                core::slice::from_raw_parts_mut(r as *mut Option<NonEmptySinStr> as *mut u8, 0)
            },
        }
    }

    #[inline(always)]
    pub fn is_inlined(&self) -> bool {
        // Implementation detail but we consider an empty string inlined
        match &self.0 {
            Some(r) => r.is_inlined(),
            None => true,
        }
    }

    #[inline(always)]
    pub fn is_heap(&self) -> bool {
        self.0.as_ref().is_some_and(NonEmptySinStr::is_heap)
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
