#![no_main]

use libfuzzer_sys::fuzz_target;
extern crate sinstr;

use sinstr::SinStr;
use std::str;

fuzz_target!(|data: Vec<Vec<u8>>| {
    // Convert valid UTF-8 inputs to SinStr
    let strings: Vec<SinStr> = data
        .iter()
        .filter_map(|v| str::from_utf8(v).ok())
        .map(SinStr::new)
        .collect();

    // Test reflexivity: a == a for all a
    for s in &strings {
        assert_eq!(s, s, "Reflexivity failed for: {:?}", s.as_str());
        assert!(s == s, "Reflexivity operator failed for: {:?}", s.as_str());

        // Test Eq consistency - if a == b, then b == a (symmetry)
        let self_eq = s == s;
        let self_eq_method = s.eq(s);
        assert_eq!(
            self_eq,
            self_eq_method,
            "Operator vs method inconsistency for: {:?}",
            s.as_str()
        );
    }

    // Test all pairs for equivalence relations
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            let a = &strings[i];
            let b = &strings[j];

            // Test symmetry: a == b implies b == a
            if a == b {
                assert!(
                    b == a,
                    "Symmetry failed: {:?} == {:?} but not reverse",
                    a.as_str(),
                    b.as_str()
                );
            } else {
                assert!(
                    !(b == a),
                    "Symmetry failed: {:?} != {:?} but reverse comparison succeeded",
                    a.as_str(),
                    b.as_str()
                );
            }

            // Test symmetry for != operator
            if a != b {
                assert!(
                    b != a,
                    "Symmetry failed for !=: {:?} != {:?} but not reverse",
                    a.as_str(),
                    b.as_str()
                );
            }
        }
    }

    // Test transitivity: a == b && b == c implies a == c
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            for k in 0..strings.len() {
                let a = &strings[i];
                let b = &strings[j];
                let c = &strings[k];

                if a == b && b == c {
                    assert!(
                        a == c,
                        "Transitivity failed: {:?} == {:?} and {:?} == {:?} but {:?} != {:?}",
                        a.as_str(),
                        b.as_str(),
                        b.as_str(),
                        c.as_str(),
                        a.as_str(),
                        c.as_str()
                    );
                }
            }
        }
    }

    // Test all pairs for storage mode and equality consistency
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            let a = &strings[i];
            let b = &strings[j];

            // Same content implies equality regardless of storage mode
            if a.as_str() == b.as_str() {
                assert!(
                    a == b,
                    "Same content but different equality: {:?} != {:?} (storage: {}, {})",
                    a.as_str(),
                    b.as_str(),
                    if a.is_inlined() {
                        "inline"
                    } else if a.is_heap() {
                        "heap"
                    } else {
                        "empty"
                    },
                    if b.is_inlined() {
                        "inline"
                    } else if b.is_heap() {
                        "heap"
                    } else {
                        "empty"
                    }
                );
            }

            // Different content implies inequality
            if a.as_str() != b.as_str() {
                assert!(
                    a != b,
                    "Different content but equal: {:?} == {:?}",
                    a.as_str(),
                    b.as_str()
                );
            }

            // Equality must be consistent with content comparison
            let str_equals = a.as_str() == b.as_str();
            let sinstr_equals = a == b;
            assert_eq!(
                str_equals,
                sinstr_equals,
                "String equality {} doesn't match SinStr equality {} for {:?} and {:?}",
                str_equals,
                sinstr_equals,
                a.as_str(),
                b.as_str()
            );
        }
    }

    // Test cross-storage comparison (inline vs heap with same semantics)
    for i in 0..strings.len().min(10) {
        for j in 0..strings.len().min(10) {
            let a = &strings[i];
            let b = &strings[j];

            // Empty strings should all be equal
            if a.is_empty() && b.is_empty() {
                assert_eq!(a, b, "Empty strings should be equal");
            }

            // Strings at boundary (7 bytes on 64-bit)
            // Both inline strings of same content should be equal
            if a.is_inlined() && b.is_inlined() && a.as_str() == b.as_str() {
                assert_eq!(a, b);
            }

            // Both heap strings of same content should be equal
            if a.is_heap() && b.is_heap() && a.as_str() == b.as_str() {
                assert_eq!(a, b);
            }
        }
    }
});
