#![no_main]

use std::{
    collections::{BTreeMap, HashMap},
    hash::RandomState,
};

use libfuzzer_sys::fuzz_target;
use sinstr::SinStr;

fuzz_target!(|data: Vec<&str>| {
    let sins = Vec::from_iter(data.into_iter().map(SinStr::new));
    let hm = HashMap::<_, _, RandomState>::from_iter(sins.iter().cloned().enumerate());

    for (i, s) in hm.iter() {
        assert_eq!(s, &sins[*i]);
    }

    // Test get operations
    for (i, s) in sins.iter().enumerate() {
        assert!(hm.contains_key(&i));
        assert_eq!(hm.get(&i), Some(s));
    }
    assert!(!hm.contains_key(&(sins.len() + 100)));

    // Test contains_key
    for i in 0..sins.len() {
        assert!(hm.contains_key(&i));
    }
    assert!(!hm.contains_key(&(sins.len() + 100)));

    // Test len and is_empty
    assert_eq!(hm.len(), sins.len());
    assert_eq!(hm.is_empty(), sins.is_empty());

    // Test keys and values iterators
    let keys: Vec<_> = hm.keys().copied().collect();
    let values: Vec<_> = hm.values().collect();
    assert_eq!(keys.len(), sins.len());
    assert_eq!(values.len(), sins.len());

    // Test capacity is at least len
    assert!(hm.capacity() >= hm.len());

    // BTreeMap tests
    let bm = BTreeMap::from_iter(sins.iter().cloned().enumerate());

    for (i, s) in bm.iter() {
        assert_eq!(s, &sins[*i]);
    }

    // Test get operations
    for (i, s) in sins.iter().enumerate() {
        assert!(bm.contains_key(&i));
        assert_eq!(bm.get(&i), Some(s));
    }
    assert!(!bm.contains_key(&(sins.len() + 100)));

    // Test contains_key
    for i in 0..sins.len() {
        assert!(bm.contains_key(&i));
    }
    assert!(!bm.contains_key(&(sins.len() + 100)));

    // Test len and is_empty
    assert_eq!(bm.len(), sins.len());
    assert_eq!(bm.is_empty(), sins.is_empty());

    // Test keys and values iterators
    let keys: Vec<_> = bm.keys().copied().collect();
    let values: Vec<_> = bm.values().collect();
    assert_eq!(keys.len(), sins.len());
    assert_eq!(values.len(), sins.len());
});
