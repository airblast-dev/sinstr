# sinstr - Small Inline String

[![Crates.io Version](https://img.shields.io/crates/v/sinstr)](https://crates.io/crates/sinstr)
[![docs.rs](https://img.shields.io/docsrs/sinstr)](https://docs.rs/sinstr)
[![Rust Version](https://img.shields.io/badge/rust-1.91+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.md)

A `no_std` string type that uses niche optimization to store short strings inline
and longer strings on the heap. `SinStr` is guaranteed to be exactly `size_of::<usize>()`
bytes, the same size as a pointer.

`SinStr` (Small Inline String, pronounced "sinister") is a compact string type designed for minimal memory
footprint. It achieves its small size through niche optimization, leveraging the fact
that aligned heap pointers always have zero low bits.

## Platform Support

| Platform | Pointer Size | Max Inline Length |
|----------|--------------|-------------------|
| 64-bit   | 8 bytes      | 7 bytes           |
| 32-bit   | 4 bytes      | 3 bytes           |
| 16-bit   | 2 bytes      | 1 byte            |

Empty strings are represented as `Option::None` and consume zero bytes of storage
beyond the `SinStr` itself.

## Comparison

| Type | Size | Max Inline | `no_std` |
|------|------|------------|----------|
| SinStr | 8 bytes | 7 bytes | ✓ |
| SmolStr | 24 bytes | 23 bytes | ✓ |
| SmartString | 24 bytes | 23 bytes | ✓ |
| CompactString | 24 bytes | 24 bytes | ✓ |
| String | 24 bytes | 0 | ✓ |
| Box<str> | 16 bytes | 0 | ✓ |

## Why Use SinStr?

1. **Minimal memory footprint**: Only `size_of::<usize>()` bytes
2. **Zero allocation for short strings**: Strings up to the platform max stay on the stack
3. **Niche optimization**: `Option<SinStr>` is also `size_of::<usize>()`
4. **`no_std` compatible**: No standard library dependencies
5. **Enum-friendly**: Works well with Rust's niche optimization in enums
6. **Efficient access**: Direct dereference for heap, inline field access for inline

## Why NOT Use SinStr?

1. **Limited inline capacity** - Only 7 bytes on 64-bit systems (3 on 32-bit, 1 on 16-bit), compared to alternatives like `compact_str` (~24 bytes) or `smartstring` (~23 bytes)
2. **Architecture-dependent limits** - The inline capacity varies by target architecture, so performance differs across platforms

## Example

```rust
use sinstr::SinStr;

// Inline storage (64-bit: up to 7 bytes)
let inline = SinStr::new("hello");
assert!(inline.is_inlined());
assert_eq!(inline.len(), 5);

// Heap storage (longer strings)
let heap = SinStr::new("hello world, this is a long string");
assert!(heap.is_heap());

// Empty string
let empty = SinStr::new("");
assert!(empty.is_empty());
```

## How It Works

The key insight is that heap pointers are always aligned to `align_of::<usize>()`.
This alignment guarantees that certain low bits are always zero:

- **64-bit systems**: 8-byte alignment → low 3 bits always zero
- **32-bit systems**: 4-byte alignment → low 2 bits always zero
- **16-bit systems**: 2-byte alignment → low 1 bit always zero

These always-zero bits create a "niche" that can store discriminant information
without increasing the type's size. Inline strings use the low bits to store the string length,
while heap strings use those bits as part of the pointer (since heap pointers are aligned,
those bits will never match an inline discriminant).

See [the documentation](https://docs.rs/sinstr) for detailed bit layout diagrams and implementation details.

## `no_std` Support

This crate is `#![no_std]` compatible. It requires the `alloc` crate for heap allocations
when strings exceed the inline capacity.

```rust
// In your Cargo.toml
[dependencies]
sinstr = "0.1.0"
```

Note: You'll need a global allocator in `no_std` environments for heap allocations.
