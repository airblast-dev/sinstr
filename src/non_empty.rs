use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt::{Debug, Display},
    hash::Hash,
    hint::assert_unchecked,
    mem::{MaybeUninit, transmute},
    num::{NonZeroU8, NonZeroUsize},
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

use alloc::alloc::{alloc, dealloc, handle_alloc_error};

use crate::{
    discriminant::{DiscriminantValues, NICHE_MAX_INT},
    likely, unlikely,
};

#[repr(C)]
pub struct HeapRepr(NonZeroUsize);

impl HeapRepr {
    #[inline]
    pub fn as_ptr(&self) -> NonNull<NonZeroUsize> {
        let p = NonNull::with_exposed_provenance(self.0);
        unsafe { assert_unchecked((p.as_ptr() as usize).is_multiple_of(align_of::<usize>())) };
        p
    }

    #[inline]
    pub fn as_ptr_mut(&mut self) -> NonNull<NonZeroUsize> {
        let p = NonNull::with_exposed_provenance(self.0);
        unsafe { assert_unchecked((p.as_ptr() as usize).is_multiple_of(align_of::<usize>())) };
        p
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
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
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
    #[cfg(target_endian = "big")]
    data: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    len: NonZeroU8,
    #[cfg(target_endian = "little")]
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
    #[cfg(target_endian = "big")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
    disc: DiscriminantValues,
    #[cfg(target_endian = "little")]
    data_or_partial_ptr: [MaybeUninit<u8>; size_of::<NonZeroUsize>() - 1],
}

impl Clone for NonEmptySinStr {
    fn clone(&self) -> Self {
        if self.is_inlined() {
            unsafe { Self::new_inline(self.as_str()) }
        } else {
            unsafe { Self::new_heap(self.as_str()) }
        }
    }

    // TODO: implement clone_from when capacity is tracked
}

impl Debug for NonEmptySinStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NonEmptySinStr")
            .field("data_or_partial_ptr", &self.data_or_partial_ptr)
            .field("disc", &self.disc)
            .finish()
    }
}

impl Display for NonEmptySinStr {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <str as Display>::fmt(self.as_str(), f)
    }
}

impl Hash for NonEmptySinStr {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for NonEmptySinStr {
    fn eq(&self, other: &Self) -> bool {
        // We check storage mode first because inline strings can only hold
        // lengths 1..=NICHE_MAX_INT while heap strings always have lengths
        // > NICHE_MAX_INT. If storage modes differ, lengths must differ,
        // so strings cannot be equal. This serves as a fast-path rejection.
        self.is_inlined() == other.is_inlined() && self.as_str() == other.as_str()
    }
}
impl Eq for NonEmptySinStr {}

impl PartialOrd for NonEmptySinStr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NonEmptySinStr {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Deref for NonEmptySinStr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
impl DerefMut for NonEmptySinStr {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl AsRef<str> for NonEmptySinStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for NonEmptySinStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for NonEmptySinStr {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl BorrowMut<str> for NonEmptySinStr {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_str_mut()
    }
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
        debug_assert!(len > 0 && len <= NICHE_MAX_INT);
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
                disc: transmute::<u8, DiscriminantValues>(len as u8),
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
        if unlikely(total_size > isize::MAX as usize) {
            panic!("NonEmptySinStr::new_heap should never exceed max size");
        }
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
            unsafe { self.get_heap_mut().as_bytes_mut() }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discriminant::NICHE_MAX_INT;
    use alloc::string::String;

    fn max_inline_string() -> String {
        "x".repeat(NICHE_MAX_INT)
    }

    fn first_heap_string() -> String {
        "y".repeat(NICHE_MAX_INT + 1)
    }

    mod constructor {
        use super::*;

        #[test]
        fn test_new_returns_none_for_empty() {
            assert!(NonEmptySinStr::new("").is_none());
        }

        #[test]
        fn test_new_inline_string() {
            for len in 1..=NICHE_MAX_INT {
                let s = "a".repeat(len);
                let nes = NonEmptySinStr::new(&s).expect("should create");
                assert!(nes.is_inlined());
                assert!(!nes.is_heap());
                assert_eq!(nes.len().get(), len);
                assert_eq!(nes.as_str(), s);
            }
        }

        #[test]
        fn test_new_heap_string() {
            let s = first_heap_string();
            let nes = NonEmptySinStr::new(&s).expect("should create");
            assert!(!nes.is_inlined());
            assert!(nes.is_heap());
            assert_eq!(nes.len().get(), NICHE_MAX_INT + 1);
            assert_eq!(nes.as_str(), s);
        }

        #[test]
        fn test_new_inline_boundary() {
            let max_inline = max_inline_string();
            let nes = NonEmptySinStr::new(&max_inline).expect("should create");
            assert!(nes.is_inlined());

            let first_heap = first_heap_string();
            let nes = NonEmptySinStr::new(&first_heap).expect("should create");
            assert!(nes.is_heap());
        }
    }

    mod unsafe_constructors {
        use super::*;

        #[test]
        fn test_new_inline_valid() {
            for len in 1..=NICHE_MAX_INT {
                let s = "x".repeat(len);
                let nes = unsafe { NonEmptySinStr::new_inline(&s) };
                assert!(nes.is_inlined());
                assert_eq!(nes.as_str(), s);
            }
        }

        #[test]
        fn test_new_heap_valid() {
            let s = "a".repeat(NICHE_MAX_INT + 5);
            let nes = unsafe { NonEmptySinStr::new_heap(&s) };
            assert!(nes.is_heap());
            assert_eq!(nes.as_str(), s);
        }
    }

    mod storage_mode {
        use super::*;

        #[test]
        fn test_is_heap_inline_string() {
            let s = "a".repeat(NICHE_MAX_INT);
            let nes = NonEmptySinStr::new(&s).expect("should create");
            assert!(!nes.is_heap());
        }

        #[test]
        fn test_is_heap_heap_string() {
            let s = first_heap_string();
            let nes = NonEmptySinStr::new(&s).expect("should create");
            assert!(nes.is_heap());
        }

        #[test]
        fn test_const_is_inlined() {
            const S: NonEmptySinStr = unsafe { NonEmptySinStr::new_inline("a") };
            assert_eq!(S.as_str(), "a");
            assert!(S.is_inlined());
        }
    }

    mod content_access {
        use super::*;

        #[test]
        fn test_len_inline() {
            for len in 1..=NICHE_MAX_INT.min(5) {
                let s = "x".repeat(len);
                let nes = NonEmptySinStr::new(&s).expect("should create");
                assert_eq!(nes.len().get(), len);
            }
        }

        #[test]
        fn test_len_heap() {
            let s = "x".repeat(100);
            let nes = NonEmptySinStr::new(&s).expect("should create");
            assert_eq!(nes.len().get(), 100);
        }

        #[test]
        fn test_as_str_inline() {
            let s = "hello";
            if NICHE_MAX_INT >= 5 {
                let nes = NonEmptySinStr::new(s).expect("should create");
                assert_eq!(nes.as_str(), s);
            }
        }

        #[test]
        fn test_as_str_heap() {
            let s = "hello world, this is a long string";
            let nes = NonEmptySinStr::new(s).expect("should create");
            assert_eq!(nes.as_str(), s);
        }

        #[test]
        fn test_as_bytes_inline() {
            let s = "abc";
            if NICHE_MAX_INT >= 3 {
                let nes = NonEmptySinStr::new(s).expect("should create");
                assert_eq!(nes.as_bytes(), s.as_bytes());
            }
        }

        #[test]
        fn test_as_bytes_heap() {
            let s = "longer string on the heap";
            let nes = NonEmptySinStr::new(s).expect("should create");
            assert_eq!(nes.as_bytes(), s.as_bytes());
        }

        #[test]
        fn test_as_str_mut_inline() {
            if NICHE_MAX_INT >= 3 {
                let mut nes = NonEmptySinStr::new("abc").expect("should create");
                let s_mut = nes.as_str_mut();
                assert_eq!(s_mut, "abc");
            }
        }

        #[test]
        fn test_as_str_mut_heap() {
            let original = "hello world";
            let mut nes = NonEmptySinStr::new(original).expect("should create");
            let s_mut = nes.as_str_mut();
            assert_eq!(s_mut, original);
        }

        #[test]
        fn test_as_bytes_mut_inline() {
            if NICHE_MAX_INT >= 3 {
                let mut nes = NonEmptySinStr::new("abc").expect("should create");
                let bytes = nes.as_bytes_mut();
                assert_eq!(bytes, b"abc");
            }
        }

        #[test]
        fn test_as_bytes_mut_heap() {
            let original = "hello world";
            let mut nes = NonEmptySinStr::new(original).expect("should create");
            let bytes = nes.as_bytes_mut();
            assert_eq!(bytes, original.as_bytes());
        }
    }

    mod unsafe_accessors {
        use super::*;

        #[test]
        fn test_get_inlined() {
            let s = "ab";
            if NICHE_MAX_INT >= 2 {
                let nes = NonEmptySinStr::new(s).expect("should create");
                let inlined = unsafe { nes.get_inlined() };
                assert_eq!(inlined.as_str(), s);
            }
        }

        #[test]
        fn test_get_inlined_mut() {
            let s = "xy";
            if NICHE_MAX_INT >= 2 {
                let mut nes = NonEmptySinStr::new(s).expect("should create");
                let inlined = unsafe { nes.get_inlined_mut() };
                assert_eq!(inlined.as_str(), s);
            }
        }

        #[test]
        fn test_get_heap() {
            let s = first_heap_string();
            let nes = NonEmptySinStr::new(&s).expect("should create");
            let heap = unsafe { nes.get_heap() };
            assert_eq!(heap.as_str(), &s);
        }

        #[test]
        fn test_get_heap_mut() {
            let s = first_heap_string();
            let mut nes = NonEmptySinStr::new(&s).expect("should create");
            let heap = unsafe { nes.get_heap_mut() };
            assert_eq!(heap.as_str(), &s);
        }

        #[test]
        fn test_heap_repr_len() {
            let len = NICHE_MAX_INT + 10;
            let s = "x".repeat(len);
            let nes = NonEmptySinStr::new(&s).expect("should create");
            let heap = unsafe { nes.get_heap() };
            assert_eq!(heap.len().get(), len);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_unicode_inline() {
            let unicode_chars = [("é", 2), ("日", 3), ("🦀", 4)];
            for (c, byte_len) in unicode_chars {
                if NICHE_MAX_INT >= byte_len {
                    let nes = NonEmptySinStr::new(c).expect("should create");
                    assert!(nes.is_inlined());
                    assert_eq!(nes.as_str(), c);
                }
            }
        }

        #[test]
        fn test_unicode_heap() {
            let s = "日本語テスト";
            let nes = NonEmptySinStr::new(s).expect("should create");
            assert_eq!(nes.as_str(), s);
            assert!(nes.is_heap());
        }

        #[test]
        fn test_unicode_max_inline() {
            if NICHE_MAX_INT >= 4 {
                let s = "🦀".repeat(NICHE_MAX_INT / 4);
                let nes = NonEmptySinStr::new(&s).expect("should create");
                assert!(nes.is_inlined());
                assert_eq!(nes.as_str(), s);
            }
        }

        #[test]
        fn test_very_long_heap_string() {
            let s = "x".repeat(10000);
            let nes = NonEmptySinStr::new(&s).expect("should create");
            assert!(nes.is_heap());
            assert_eq!(nes.len().get(), 10000);
            assert_eq!(nes.as_str(), s);
        }
    }

    mod trait_impls {
        use super::*;
        use alloc::borrow::Borrow;
        use core::hash::{Hash, Hasher};

        mod clone_tests {
            use super::*;

            #[test]
            fn test_clone_inline() {
                let s = "abc";
                if NICHE_MAX_INT >= 3 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    let cloned = nes.clone();
                    assert_eq!(cloned.as_str(), s);
                    assert!(cloned.is_inlined());
                }
            }

            #[test]
            fn test_clone_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                let cloned = nes.clone();
                assert_eq!(cloned.as_str(), &s);
                assert!(cloned.is_heap());
            }

            #[test]
            fn test_clone_max_inline() {
                let s = max_inline_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                let cloned = nes.clone();
                assert_eq!(cloned.as_str(), &s);
                assert!(cloned.is_inlined());
            }

            #[test]
            fn test_clone_preserves_content() {
                let original = "hello";
                if NICHE_MAX_INT >= 5 {
                    let nes = NonEmptySinStr::new(original).expect("should create");
                    let cloned = nes.clone();
                    assert_eq!(original, cloned.as_str());
                }
            }
        }

        mod display_tests {
            use super::*;

            #[test]
            fn test_display_inline() {
                let s = "hello";
                if NICHE_MAX_INT >= 5 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    let displayed = alloc::format!("{}", nes);
                    assert_eq!(displayed, s);
                }
            }

            #[test]
            fn test_display_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                let displayed = alloc::format!("{}", nes);
                assert_eq!(displayed, s);
            }

            #[test]
            fn test_display_unicode() {
                let s = "日本語";
                let nes = NonEmptySinStr::new(s).expect("should create");
                let displayed = alloc::format!("{}", nes);
                assert_eq!(displayed, s);
            }
        }

        mod hash_tests {
            use super::*;
            extern crate std;
            use std::hash::DefaultHasher;

            fn calculate_hash<T: Hash>(value: &T) -> u64 {
                let mut hasher = DefaultHasher::new();
                value.hash(&mut hasher);
                hasher.finish()
            }

            #[test]
            fn test_hash_consistency() {
                let s = "abc";
                if NICHE_MAX_INT >= 3 {
                    let nes1 = NonEmptySinStr::new(s).expect("should create");
                    let nes2 = NonEmptySinStr::new(s).expect("should create");
                    assert_eq!(calculate_hash(&nes1), calculate_hash(&nes2));
                }
            }

            #[test]
            fn test_hash_different() {
                if NICHE_MAX_INT >= 3 {
                    let nes1 = NonEmptySinStr::new("abc").expect("should create");
                    let nes2 = NonEmptySinStr::new("xyz").expect("should create");
                    assert_ne!(calculate_hash(&nes1), calculate_hash(&nes2));
                }
            }

            #[test]
            fn test_hash_inline_vs_heap_different() {
                let short = "ab";
                let long_suffix = "x".repeat(NICHE_MAX_INT);
                let content = alloc::format!("{}{}", short, long_suffix);

                let nes_heap = NonEmptySinStr::new(&content).expect("should create");
                let str_ref: &str = nes_heap.as_str();
                let nes_inline =
                    NonEmptySinStr::new(str_ref.get(0..2).unwrap()).expect("should create");

                assert_ne!(
                    calculate_hash(&nes_heap),
                    calculate_hash(&nes_inline),
                    "Different content should have different hashes"
                );
            }
        }

        mod eq_tests {
            use super::*;

            #[test]
            fn test_eq_same_content_inline() {
                let s = "test";
                if NICHE_MAX_INT >= 4 {
                    let a = NonEmptySinStr::new(s).expect("should create");
                    let b = NonEmptySinStr::new(s).expect("should create");
                    assert_eq!(a, b);
                }
            }

            #[test]
            fn test_eq_same_content_heap() {
                let s = first_heap_string();
                let a = NonEmptySinStr::new(&s).expect("should create");
                let b = NonEmptySinStr::new(&s).expect("should create");
                assert_eq!(a, b);
            }

            #[test]
            fn test_ne_different_content() {
                if NICHE_MAX_INT >= 3 {
                    let a = NonEmptySinStr::new("abc").expect("should create");
                    let b = NonEmptySinStr::new("xyz").expect("should create");
                    assert_ne!(a, b);
                }
            }

            #[test]
            fn test_eq_inline_and_heap_same_storage_mode_check() {
                let s1 = "a".repeat(NICHE_MAX_INT);
                let s2 = s1.clone();
                let a = NonEmptySinStr::new(&s1).expect("should create");
                let b = NonEmptySinStr::new(&s2).expect("should create");
                assert!(a.is_inlined());
                assert!(b.is_inlined());
                assert_eq!(a, b);
            }
        }

        mod ord_tests {
            use super::*;
            use core::cmp::Ordering;

            #[test]
            fn test_ord_less() {
                if NICHE_MAX_INT >= 2 {
                    let a = NonEmptySinStr::new("ab").expect("should create");
                    let b = NonEmptySinStr::new("cd").expect("should create");
                    assert_eq!(a.cmp(&b), Ordering::Less);
                    assert!(a < b);
                }
            }

            #[test]
            fn test_ord_equal() {
                let s = "test";
                if NICHE_MAX_INT >= 4 {
                    let a = NonEmptySinStr::new(s).expect("should create");
                    let b = NonEmptySinStr::new(s).expect("should create");
                    assert_eq!(a.cmp(&b), Ordering::Equal);
                    assert!(a <= b);
                    assert!(a >= b);
                }
            }

            #[test]
            fn test_ord_greater() {
                if NICHE_MAX_INT >= 2 {
                    let a = NonEmptySinStr::new("yz").expect("should create");
                    let b = NonEmptySinStr::new("ab").expect("should create");
                    assert_eq!(a.cmp(&b), Ordering::Greater);
                    assert!(a > b);
                }
            }

            #[test]
            fn test_ord_cross_storage() {
                let short = "ab";
                let long = "cd";
                if NICHE_MAX_INT >= 2 && long.len() <= NICHE_MAX_INT {
                    let a = NonEmptySinStr::new(short).expect("should create");
                    let b = NonEmptySinStr::new(long).expect("should create");
                    assert!(a < b);
                }
            }
        }

        mod deref_tests {
            use super::*;

            #[test]
            fn test_deref_inline() {
                let s = "hello";
                if NICHE_MAX_INT >= 5 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    assert_eq!(&*nes, s);
                }
            }

            #[test]
            fn test_deref_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                assert_eq!(&*nes, &s);
            }

            #[test]
            fn test_deref_methods() {
                let s = "test";
                if NICHE_MAX_INT >= 4 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    assert_eq!(nes.len().get(), s.len());
                    assert!(nes.starts_with("te"));
                    assert!(nes.ends_with("st"));
                }
            }

            #[test]
            fn test_deref_mut_inline() {
                if NICHE_MAX_INT >= 3 {
                    let nes = NonEmptySinStr::new("abc").expect("should create");
                    assert_eq!(&*nes, "abc");
                }
            }

            #[test]
            fn test_deref_mut_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                assert_eq!(&*nes, &s);
            }
        }

        mod as_ref_tests {
            use super::*;

            #[test]
            fn test_as_ref_str_inline() {
                let s = "test";
                if NICHE_MAX_INT >= 4 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    let as_str: &str = nes.as_ref();
                    assert_eq!(as_str, s);
                }
            }

            #[test]
            fn test_as_ref_str_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                let as_str: &str = nes.as_ref();
                assert_eq!(as_str, &s);
            }

            #[test]
            fn test_as_ref_bytes_inline() {
                let s = "abc";
                if NICHE_MAX_INT >= 3 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    let as_bytes: &[u8] = nes.as_ref();
                    assert_eq!(as_bytes, s.as_bytes());
                }
            }

            #[test]
            fn test_as_ref_bytes_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                let as_bytes: &[u8] = nes.as_ref();
                assert_eq!(as_bytes, s.as_bytes());
            }
        }

        mod borrow_tests {
            use super::*;

            #[test]
            fn test_borrow_str_inline() {
                let s = "test";
                if NICHE_MAX_INT >= 4 {
                    let nes = NonEmptySinStr::new(s).expect("should create");
                    let borrowed: &str = nes.borrow();
                    assert_eq!(borrowed, s);
                }
            }

            #[test]
            fn test_borrow_str_heap() {
                let s = first_heap_string();
                let nes = NonEmptySinStr::new(&s).expect("should create");
                let borrowed: &str = nes.borrow();
                assert_eq!(borrowed, &s);
            }

            #[test]
            fn test_borrow_mut_str_inline() {
                let s = "abc";
                if NICHE_MAX_INT >= 3 {
                    let mut nes = NonEmptySinStr::new(s).expect("should create");
                    let borrowed: &mut str = nes.borrow_mut();
                    assert_eq!(borrowed, s);
                }
            }

            #[test]
            fn test_borrow_mut_str_heap() {
                let s = first_heap_string();
                let mut nes = NonEmptySinStr::new(&s).expect("should create");
                let borrowed: &mut str = nes.borrow_mut();
                assert_eq!(borrowed, &s);
            }
        }
    }
}
