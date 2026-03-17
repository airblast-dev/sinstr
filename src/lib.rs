use core::str;
use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error},
    mem::{MaybeUninit, size_of, transmute, transmute_copy},
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
            let mut buf = [MaybeUninit::uninit(); size_of::<NonZeroUsize>() - 1];
            for (i, &b) in s.as_bytes().iter().enumerate() {
                buf[i] = MaybeUninit::new(b);
            }
            unsafe {
                Self(Some(Repr {
                    _align: [],
                    disc: transmute::<u8, discriminant::DiscriminantValues>(len as u8),
                    data_or_partial_ptr: buf,
                }))
            }
        } else {
            let total_size = size_of::<usize>()
                .checked_add(len)
                .expect("string too large");
            let layout = Layout::from_size_align(total_size, align_of::<usize>()).unwrap();

            // SAFETY: layout size > 0 because len > NICHE_MAX_INT > 0
            let Some(ptr) = NonNull::new(unsafe { alloc(layout) }) else {
                handle_alloc_error(layout)
            };

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
}
