use core::{borrow::{Borrow, BorrowMut}, convert::Infallible, fmt::Display, ops::{Deref, DerefMut}, str::FromStr};

use crate::non_empty::NonEmptySinStr;

#[repr(transparent)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SinStr(Option<NonEmptySinStr>);

impl Default for SinStr {
    #[inline(always)]
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Display for SinStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = self
            .0
            .as_ref()
            .map(NonEmptySinStr::as_str)
            .unwrap_or_default();

        <str as Display>::fmt(s, f)
    }
}

impl Deref for SinStr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
impl DerefMut for SinStr {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl AsRef<str> for SinStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for SinStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for SinStr {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl BorrowMut<str> for SinStr {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_str_mut()
    }
}

impl From<&str> for SinStr {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl FromStr for SinStr {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl SinStr {
    pub const EMPTY: Self = Self(None);

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
    use super::SinStr;

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
