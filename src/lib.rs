use core::str;
use std::{
    mem::{MaybeUninit, transmute, transmute_copy},
    num::{NonZeroU8, NonZeroUsize},
};

mod discriminant;
pub use discriminant::DiscriminantValues;

use crate::discriminant::NICHE_MAX_INT;

#[repr(C)]
struct HeapRepr {
    len: usize,
    data: [u8; 0],
    // trailing bytes
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

    fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
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
        len <= NICHE_MAX_INT
    }

    pub fn is_heap(&self) -> bool {
        !self.is_inlined()
    }

    pub fn len(&self) -> usize {
        if self.is_inlined() {
            self.disc as usize
        } else {
            unsafe { self.get_heap().len }
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
            let hp: *const HeapRepr =
                core::ptr::with_exposed_provenance_mut(usize::from_ne_bytes(transmute_copy::<
                    Repr,
                    [u8; size_of::<usize>()],
                >(
                    self
                )));
            hp.as_ref().unwrap_unchecked()
        }
    }

    fn get_inlined(&self) -> &InlinedRepr {
        unsafe { transmute(self) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        if self.is_inlined() {
            self.get_inlined().as_bytes()
        } else {
            todo!()
        }
    }

    pub fn as_str(&self) -> &str {
        if self.is_inlined() {
            self.get_inlined().as_str()
        } else {
            todo!()
        }
    }
}

const _: () = assert!(size_of::<Repr>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<Repr>>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<Repr>>() >= align_of::<usize>());

#[repr(transparent)]
struct SinStr(Repr);

impl SinStr {
    pub fn new(s: &str) -> Option<Self> {
        let len = s.len();
        if len == 0 {
            return None;
        }

        if NICHE_MAX_INT >= len {
            let mut buf = [MaybeUninit::uninit(); size_of::<NonZeroUsize>() - 1];
            for (i, &b) in s.as_bytes().iter().enumerate() {
                buf[i] = MaybeUninit::new(b);
            }
            unsafe {
                Some(Self(Repr {
                    _align: [],
                    disc: transmute::<u8, discriminant::DiscriminantValues>(len as u8),
                    data_or_partial_ptr: buf,
                }))
            }
        } else {
            todo!()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::discriminant::NICHE_BITS;

    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn test_empty_string_returns_none() {
        assert!(SinStr::new("").is_none());
    }

    #[test]
    fn test_single_char_inlined() {
        let s = SinStr::new("a").unwrap();
        assert_eq!(s.0.len(), 1);
        assert!(s.0.is_inlined());
        assert!(!s.0.is_heap());
        assert_eq!(s.0.as_str(), "a");
        assert_eq!(s.0.as_bytes(), b"a");
    }

    #[test]
    fn test_all_inline_lengths() {
        for len in 1..=NICHE_MAX_INT {
            let input: String = "x".repeat(len);
            let s = SinStr::new(&input).unwrap();
            assert_eq!(s.0.len(), len);
            assert!(s.0.is_inlined());
            assert_eq!(s.0.as_str(), input.as_str());
            assert_eq!(s.0.as_bytes(), input.as_bytes());
        }
    }

    #[test]
    fn test_niche_bits_correct() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(NICHE_BITS, 3);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(NICHE_BITS, 2);
        #[cfg(target_pointer_width = "16")]
        assert_eq!(NICHE_BITS, 1);
    }

    #[test]
    fn test_niche_max_int_correct() {
        assert_eq!(NICHE_MAX_INT, (1usize << NICHE_BITS) - 1);
    }

    #[test]
    fn test_inline_various_characters() {
        let test_cases = ["a", "ab", "abc123", "!@#$%", "hello", "world"];
        for &input in &test_cases {
            if input.len() <= NICHE_MAX_INT {
                let s = SinStr::new(input).unwrap();
                assert_eq!(s.0.len(), input.len());
                assert!(s.0.is_inlined());
                assert_eq!(s.0.as_str(), input);
                assert_eq!(s.0.as_bytes(), input.as_bytes());
            }
        }
    }

    #[test]
    fn test_inline_null_byte() {
        let s = SinStr::new("\0").unwrap();
        assert_eq!(s.0.len(), 1);
        assert!(s.0.is_inlined());
        assert_eq!(s.0.as_str(), "\0");
        assert_eq!(s.0.as_bytes(), b"\0");
    }

    #[test]
    fn test_inline_multi_byte_utf8() {
        let s = SinStr::new("é").unwrap();
        assert_eq!(s.0.len(), 2);
        assert!(s.0.is_inlined());
        assert_eq!(s.0.as_str(), "é");
        assert_eq!(s.0.as_bytes(), "é".as_bytes());

        if NICHE_MAX_INT >= 3 {
            let s = SinStr::new("日").unwrap();
            assert_eq!(s.0.len(), 3);
            assert!(s.0.is_inlined());
            assert_eq!(s.0.as_str(), "日");
            assert_eq!(s.0.as_bytes(), "日".as_bytes());
        }
    }

    #[test]
    fn test_discriminant_values_inline_range() {
        let s = SinStr::new("x").unwrap();
        let disc_value = s.0.disc as u8;
        assert!(disc_value <= NICHE_MAX_INT as u8);
    }
}
