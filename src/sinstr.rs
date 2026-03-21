use core::{
    borrow::{Borrow, BorrowMut},
    convert::Infallible,
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

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
    #[inline(always)]
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
    pub const fn as_str(&self) -> &str {
        match &self.0 {
            Some(r) => r.as_str(),
            None => "",
        }
    }

    /// Returns the string as a `&mut str`.
    #[inline(always)]
    pub const fn as_str_mut(&mut self) -> &mut str {
        match &mut self.0 {
            Some(r) => r.as_str_mut(),
            None => {
                const S: &mut str = match str::from_utf8_mut(&mut []) {
                    Ok(s) => s,
                    Err(_) => panic!("should never fail to create empty string at compile time"),
                };

                S
            }
        }
    }

    /// Returns the string as a slice of bytes.
    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
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
    #[inline(always)]
    pub const unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        match &mut self.0 {
            Some(r) => unsafe { r.as_bytes_mut() },
            None => &mut [],
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
    fn test_empty() {
        let s = SinStr::new("");
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.as_str(), "");
        assert_eq!(s.as_bytes(), b"");
        assert!(s.is_inlined());
        assert!(!s.is_heap());
    }

    #[test]
    fn test_empty_constant() {
        assert!(SinStr::EMPTY.is_empty());
        assert_eq!(SinStr::EMPTY.len(), 0);
    }

    #[test]
    fn test_default() {
        let s: SinStr = SinStr::default();
        assert!(s.is_empty());
    }

    #[test]
    fn test_from_str() {
        use core::str::FromStr;
        assert!(SinStr::from_str("").unwrap().is_empty());
        assert_eq!(SinStr::from_str("test").unwrap().as_str(), "test");
    }

    #[test]
    fn test_display() {
        assert_eq!(alloc::format!("{}", SinStr::new("")), "");
        assert_eq!(alloc::format!("{}", SinStr::new("hello")), "hello");
    }
}
