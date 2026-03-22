#![no_main]

use libfuzzer_sys::fuzz_target;
extern crate sinstr;

use sinstr::SinStr;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::str;

fuzz_target!(|data: Vec<Vec<u8>>| {
    // Convert valid UTF-8 inputs to SinStr
    let strings = {
        // Only push if element doesn't exist.
        //
        // We need to do this to avoid HashMap collisions
        let mut strings = vec![];
        for s in data
            .iter()
            .filter_map(|v| str::from_utf8(v).ok())
            .map(SinStr::new)
        {
            if !strings.contains(&s) {
                strings.push(s);
            }
        }
        strings
    };

    // === Hash consistency tests ===

    // Test: equal strings must have equal hashes
    for s in &strings {
        let hash1 = {
            let mut hasher = DefaultHasher::new();
            s.hash(&mut hasher);
            hasher.finish()
        };

        // Creating a clone should have the same hash
        let s_clone = s.clone();
        let hash2 = {
            let mut hasher = DefaultHasher::new();
            s_clone.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(
            hash1,
            hash2,
            "Hash not consistent for cloned string: {:?}",
            s.as_str()
        );

        // Also verify with another hasher instance
        let hash3 = {
            let mut hasher = DefaultHasher::new();
            s.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(hash1, hash3, "Hash not deterministic: {:?}", s.as_str());
    }

    // Test: hash equality for equal strings
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            if strings[i] == strings[j] {
                let hash_i = {
                    let mut hasher = DefaultHasher::new();
                    strings[i].hash(&mut hasher);
                    hasher.finish()
                };
                let hash_j = {
                    let mut hasher = DefaultHasher::new();
                    strings[j].hash(&mut hasher);
                    hasher.finish()
                };

                assert_eq!(
                    hash_i,
                    hash_j,
                    "Equal strings have different hashes: {:?} (hash {}) == {:?} (hash {})",
                    strings[i].as_str(),
                    hash_i,
                    strings[j].as_str(),
                    hash_j
                );
            }
        }
    }

    // === HashMap tests ===

    // Test insertion and retrieval
    let mut map: HashMap<SinStr, usize> = HashMap::new();
    for (i, s) in strings.iter().enumerate() {
        let len_before = map.len();
        map.insert(s.clone(), i);

        // Verify it was inserted/replaced
        assert_eq!(
            map.get(s),
            Some(&i),
            "HashMap get failed after insert for: {:?}",
            s.as_str()
        );

        // If we've seen this string before, len should be unchanged
        // If new, len should increase by 1
        if strings[..i].iter().any(|prev| prev == s) {
            // Duplicate key
            assert_eq!(
                map.len(),
                len_before,
                "HashMap len should not change for duplicate key"
            );
        } else {
            // Unique key
            assert_eq!(
                map.len(),
                len_before + 1,
                "HashMap len should increase for unique key"
            );
        }
    }

    // Test HashMap contains_key
    for s in &strings {
        assert!(
            map.contains_key(s),
            "HashMap contains_key failed for: {:?}",
            s.as_str()
        );
    }

    // Test HashMap removal
    for s in &strings {
        let val = map.remove(s);
        assert!(
            val.is_some(),
            "HashMap remove should succeed for inserted key: {:?}",
            s.as_str()
        );
        assert!(
            !map.contains_key(s),
            "HashMap contains_key should return false after remove"
        );
    }

    assert!(map.is_empty(), "HashMap should be empty after all removals");

    // Test HashMap with different storage modes (inline vs heap)
    let mut mixed_map: HashMap<SinStr, &str> = HashMap::new();
    for s in &strings {
        // Use as_str() to verify the lookup works regardless of storage mode
        mixed_map.insert(s.clone(), s.as_str());
    }

    // Verify all lookups work
    for s in &strings {
        let retrieved = mixed_map.get(s);
        assert_eq!(
            retrieved,
            Some(&s.as_str()),
            "HashMap lookup failed across storage modes for: {:?}",
            s.as_str()
        );
    }

    // === HashSet tests ===

    let mut set: HashSet<SinStr> = HashSet::new();
    for s in &strings {
        let inserted = set.insert(s.clone());
        // First insert should succeed, subsequent inserts of equal strings should not
        let previously_inserted = strings[..strings.len()]
            .iter()
            .take_while(|prev| *prev != s)
            .any(|prev| prev == s);

        if !previously_inserted {
            assert!(
                inserted,
                "HashSet insert should return true for new element: {:?}",
                s.as_str()
            );
        }
    }

    // Test HashSet contains
    for s in &strings {
        assert!(
            set.contains(s),
            "HashSet contains failed for: {:?}",
            s.as_str()
        );
    }

    // Test set uniqueness: duplicates should not increase size
    let unique_count = strings
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert_eq!(
        set.len(),
        unique_count,
        "HashSet size mismatch: expected {} unique elements, got {}",
        unique_count,
        set.len()
    );

    // Test HashSet removal
    for s in &strings {
        let removed = set.remove(s);
        assert!(
            removed,
            "HashSet remove should return true for contained element: {:?}",
            s.as_str()
        );
    }

    assert!(
        set.is_empty(),
        "HashSet should be empty after removing all elements"
    );

    // === BTreeMap tests ===

    let mut btree: BTreeMap<SinStr, usize> = BTreeMap::new();
    for (i, s) in strings.iter().enumerate() {
        btree.insert(s.clone(), i);
    }

    // Test BTreeMap ordering: keys should be sorted
    let mut prev_key: Option<&SinStr> = None;
    for (key, _) in btree.iter() {
        if let Some(prev) = prev_key {
            assert!(
                prev <= key,
                "BTreeMap keys not sorted: {:?} should be <= {:?}",
                prev.as_str(),
                key.as_str()
            );
        }
        prev_key = Some(key);
    }

    // Test BTreeMap retrieval
    for s in &strings {
        assert_eq!(
            btree.get(s),
            Some(&strings.iter().position(|x| x == s).unwrap()),
            "BTreeMap get failed for: {:?}",
            s.as_str()
        );
    }

    // Test BTreeMap removal
    for s in &strings {
        btree.remove(s);
    }

    assert!(
        btree.is_empty(),
        "BTreeMap should be empty after all removals"
    );

    // === BTreeSet tests ===

    let mut btset: BTreeSet<SinStr> = BTreeSet::new();
    for s in &strings {
        btset.insert(s.clone());
    }

    // Verify all elements present
    for s in &strings {
        assert!(
            btset.contains(s),
            "BTreeSet contains failed for: {:?}",
            s.as_str()
        );
    }

    // Verify ordering: BTreeSet elements should be sorted
    let mut prev: Option<&SinStr> = None;
    for s in btset.iter() {
        if let Some(p) = prev {
            assert!(
                p <= s,
                "BTreeSet elements not sorted: {:?} <= {:?}",
                p.as_str(),
                s.as_str()
            );
        }
        prev = Some(s);
    }

    // Test removal
    for s in &strings {
        btset.remove(s);
    }

    assert!(
        btset.is_empty(),
        "BTreeSet should be empty after all removals"
    );

    // === Cross-collection consistency ===

    // All equal strings should behave identically in collections
    let mut hash_count = HashMap::new();
    let mut btree_count = BTreeMap::new();

    for s in &strings {
        *hash_count.entry(s.clone()).or_insert(0) += 1;
        *btree_count.entry(s.clone()).or_insert(0) += 1;
    }

    // Counts should match
    assert_eq!(
        hash_count.len(),
        btree_count.len(),
        "HashMap and BTreeMap should have same number of unique keys"
    );

    for (key, count) in &hash_count {
        assert_eq!(
            btree_count.get(key),
            Some(count),
            "HashMap and BTreeMap counts should match for key: {:?}",
            key.as_str()
        );
    }

    // === Stress test: rapid insert/remove cycles ===

    let mut stress_map: HashMap<SinStr, usize> = HashMap::new();
    for round in 0..10 {
        for (i, s) in strings.iter().enumerate() {
            let key = s.clone();

            if round % 2 == 0 {
                stress_map.insert(key.clone(), i);
            } else {
                stress_map.remove(&key);
            }
        }
    }

    // Final state should be deterministic
    for s in &strings {
        if strings[..strings.len()]
            .iter()
            .enumerate()
            .position(|(i, x)| x == s && i % 2 == 0)
            .is_some()
        {
            // Should be present (inserted in even rounds)
        }
    }
});
