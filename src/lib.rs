//! # sinstr - Small Inline String
//!
//! A `no_std` string type that uses niche optimization to store short strings inline
//! and longer strings on the heap. `SinStr` is guaranteed to be exactly `size_of::<usize>()`
//! bytes, the same size as a pointer.
//!
//! ## Overview
//!
//! `SinStr` (Small Inline String, pronounced "sinister") is a compact string type designed for minimal memory
//! footprint. It achieves its small size through niche optimization, leveraging the fact
//! that aligned heap pointers always have zero low bits.
//!
//! ### Note
//!
//! The documentation is written assuming little endian order. The library itself is compatible with both little and big endian architectures.
//!
//! | Platform | Pointer Size | Max Inline Length |
//! |----------|--------------|-------------------|
//! | 64-bit   | 8 bytes      | 7 bytes           |
//! | 32-bit   | 4 bytes      | 3 bytes           |
//! | 16-bit   | 2 bytes      | 1 byte            |
//!
//! Empty strings are represented as [`Option::None`] and consume zero bytes of storage
//! beyond the [`SinStr`] itself.
//!
//! ## How It Works
//!
//! The key insight is that heap pointers are always aligned to `align_of::<usize>()`.
//! This alignment guarantees that certain low bits are always zero:
//!
//! - **64-bit systems**: 8-byte alignment → low 3 bits always zero
//! - **32-bit systems**: 4-byte alignment → low 2 bits always zero
//! - **16-bit systems**: 2-byte alignment → low 1 bit always zero
//!
//! These always-zero bits create a "niche" that can store discriminant information
//! without increasing the type's size. The `NonEmptySinStr` struct overlays this niche:
//!
//! ### Bit Layout (64-bit, little-endian)
//!
//! **Empty string (`Option<NonEmptySinStr>` = None):**
//!
//! ```text
//! All bits zero: 0x0000_0000_0000_0000
//! ```
//!
//! **Inline string "abc" (length = 3):**
//!
//! ```text
//! Byte:  [0] [1] [2] [3] [4] [5] [6] [7]
//! Value: 'a' 'b' 'c'  ?   ?   ?   ?  0x03
//! ```
//!
//! The last byte stores the discriminant (0x03), which equals the string length.
//! Bytes [3-6] are MaybeUninit and contain unspecified values.
//!
//! **Heap string "hello world" (11 bytes):**
//!
//! ```text
//! NonEmptySinStr is a transmuted heap pointer:
//! ┌────────────────────────────────────────────────────────┐
//! │  Pointer value (e.g., 0x5555_5555_5508)                │
//! └────────────────────────────────────────────────────────┘
//!
//! Heap memory layout:
//! ┌─────────────┬──────────────────────────────────────────────────┐
//! │ len: 11     │ data: 'h' 'e' 'l' 'l' 'o' ' ' 'w' 'o' 'r' 'l' 'd'│
//! └─────────────┴──────────────────────────────────────────────────┘
//! ```
//!
//! When interpreting the pointer as `NonEmptySinStr`, the discriminant field reads
//! the least significant byte. Since heap pointers are 8-byte aligned, this value is always
//! a multiple of 8 (0, 8, 16, ...). The `is_inlined()` check uses
//! `disc <= NICHE_MAX_INT` to distinguish inline strings (discriminant 1-7)
//! from heap pointers (discriminant == 0 is None/empty, discriminant >= 8 is heap).
//!
//! ## Discriminant Detection
//!
//! The inline/heap distinction is determined by reading the last byte as a
//! discriminant value:
//!
//! | Discriminant          | Meaning                         |
//! |-----------------------|---------------------------------|
//! | `1..=NICHE_MAX_INT`   | Inline string (value = length)  |
//! | `> NICHE_MAX_INT or equal to 0`     | LSB of heap pointer |
//!
//! ## Performance Characteristics
//!
//! Inline strings have zero heap allocation overhead and are very cheap to construct. Heap strings require
//! allocation similar to `Box<str>`.
//!
//! ## Why Use SinStr?
//!
//! 1. **Minimal memory footprint**: Only `size_of::<usize>()` bytes
//! 2. **Zero allocation for short strings**: Strings up to the platform max stay on the stack
//! 3. **Niche optimization**: [`Option<SinStr>`] is also `size_of::<usize>()`
//! 4. **`no_std` compatible**: No standard library dependencies
//! 5. **Enum-friendly**: Works well with Rust's niche optimization in enums
//! 6. **Efficient access**: Direct dereference for heap, inline field access for inline
//!
//! ## Why NOT Use SinStr?
//!
//! 1. **Limited inline capacity** - Only 7 bytes on 64-bit systems (3 on 32-bit, 1 on 16-bit), compared to alternatives like `compact_str` (~24 bytes) or `smartstring` (~23 bytes)
//! 2. **Architecture-dependent limits** - The inline capacity varies by target architecture, so performance differs across platforms
//!
//! ## Example
//!
//! ```rust
//! use sinstr::SinStr;
//!
//! // Inline storage (64-bit: up to 7 bytes)
//! let inline = SinStr::new("hello");
//! assert!(inline.is_inlined());
//! assert_eq!(inline.len(), 5);
//!
//! // Heap storage (longer strings)
//! let heap = SinStr::new("hello world, this is a long string");
//! assert!(heap.is_heap());
//!
//! // Empty string
//! let empty = SinStr::new("");
//! assert!(empty.is_empty());
//! ```

#![no_std]
extern crate alloc;

pub mod discriminant;
mod literal_macro;
mod non_empty;
mod sinstr;

pub use non_empty::*;
pub use sinstr::*;

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
