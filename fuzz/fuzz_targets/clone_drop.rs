#![no_main]

use libfuzzer_sys::fuzz_target;
extern crate sinstr;

use sinstr::{SinStr, discriminant::NICHE_MAX_INT};
use std::str;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = str::from_utf8(data) {
        // Test 1: Basic clone and drop
        let original = SinStr::new(s);
        let cloned = original.clone();

        // Verify clone is correct
        assert_eq!(original.as_str(), cloned.as_str());
        assert_eq!(original.len(), cloned.len());
        assert_eq!(original.is_inlined(), cloned.is_inlined());
        assert_eq!(original.is_heap(), cloned.is_heap());
        assert_eq!(original.is_empty(), cloned.is_empty());

        // Test 2: Clone while original dropped
        let original2 = SinStr::new(s);
        let cloned2 = original2.clone();
        drop(original2);
        // Use cloned2 after original dropped - tests for use-after-free
        assert_eq!(cloned2.as_str(), s);
        assert_eq!(cloned2.len(), s.len());

        // Test 3: Multiple clones and drops in sequence
        let original3 = SinStr::new(s);
        let clone1 = original3.clone();
        let clone2 = original3.clone();
        let clone3 = original3.clone();

        // Drop original first (tests heap reference counting)
        drop(original3);

        // Verify all clones still valid
        assert_eq!(clone1.as_str(), s);
        assert_eq!(clone2.as_str(), s);
        assert_eq!(clone3.as_str(), s);

        // Drop clones in different order
        drop(clone1);
        assert_eq!(clone2.as_str(), s);
        assert_eq!(clone3.as_str(), s);

        drop(clone3);
        assert_eq!(clone2.as_str(), s);

        // Test 4: Mixed inline/heap strings
        if s.len() <= NICHE_MAX_INT {
            // Inline string
            assert!(original.is_inlined() || original.is_empty());
            assert!(!original.is_heap());
            let inline_clone = original.clone();
            assert_eq!(inline_clone.as_str(), s);
        } else {
            // Heap string
            assert!(original.is_heap());
            assert!(!original.is_inlined());
            let heap_clone = original.clone();
            assert_eq!(heap_clone.as_str(), s);
        }

        // Test 5: Clone from clone
        let first_clone = SinStr::new(s).clone();
        let second_clone = first_clone.clone();
        let third_clone = second_clone.clone();

        assert_eq!(first_clone.as_str(), s);
        assert_eq!(second_clone.as_str(), s);
        assert_eq!(third_clone.as_str(), s);

        drop(first_clone);
        drop(second_clone);
        assert_eq!(third_clone.as_str(), s);

        // Test 6: Stress test - many clones in vector
        let original4 = SinStr::new(s);
        let mut clones: Vec<SinStr> = (0..100).map(|_| original4.clone()).collect();

        drop(original4);

        // Verify all clones still valid
        for clone in &clones {
            assert_eq!(clone.as_str(), s);
        }

        // Drop half, verify rest
        clones.drain(0..50);
        for clone in &clones {
            assert_eq!(clone.as_str(), s);
        }
    }
});
