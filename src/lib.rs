use std::{
    alloc::{self, Layout},
    mem::{MaybeUninit, transmute},
    num::NonZeroUsize,
    ptr::NonNull,
    slice,
};

mod discriminant;
pub use discriminant::DiscriminantValues;

use crate::discriminant::{NICHE_BITS, NICHE_MAX_INT};

#[repr(C)]
struct HeapRepr {
    len: usize,
    // trailing bytes
}

#[repr(C)]
struct InlinedRepr {
    data: [u8; size_of::<NonZeroUsize>() - 1],
    __pad: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
            let hp: *const HeapRepr = core::ptr::with_exposed_provenance_mut(
                usize::from_ne_bytes(transmute::<Repr, [u8; size_of::<usize>()]>(*self))
                    & !NICHE_MAX_INT,
            );
            hp.as_ref().unwrap_unchecked()
        }
    }
}

const _: () = assert!(size_of::<Repr>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<Repr>>() == size_of::<usize>());
const _: () = assert!(size_of::<Option<Repr>>() >= align_of::<usize>());

#[repr(transparent)]
struct SinStr(Repr);
