#![no_main]

use libfuzzer_sys::fuzz_target;
extern crate sinstr;

use sinstr::SinStr;
use std::cmp::Ordering;
use std::str;

fuzz_target!(|data: Vec<Vec<u8>>| {
    // Convert valid UTF-8 inputs to SinStr
    let strings: Vec<SinStr> = data
        .iter()
        .filter_map(|v| str::from_utf8(v).ok())
        .map(SinStr::new)
        .collect();

    // Test reflexivity for ordering: a.cmp(a) == Ordering::Equal
    for s in &strings {
        assert_eq!(
            s.cmp(s),
            Ordering::Equal,
            "Reflexivity failed for ordering: {:?}",
            s.as_str()
        );
        assert_eq!(
            s.partial_cmp(s),
            Some(Ordering::Equal),
            "Partial reflexivity failed: {:?}",
            s.as_str()
        );
        assert!(
            !(s < s),
            "Self should not be less than self: {:?}",
            s.as_str()
        );
        assert!(
            !(s > s),
            "Self should not be greater than self: {:?}",
            s.as_str()
        );
        assert!(
            s <= s,
            "Self should be less than or equal to self: {:?}",
            s.as_str()
        );
        assert!(
            s >= s,
            "Self should be greater than or equal to self: {:?}",
            s.as_str()
        );
    }

    // Test all pairs for ordering consistency
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            let a = &strings[i];
            let b = &strings[j];

            // PartialOrd must be consistent with Ord
            let partial_result = a.partial_cmp(b);
            let full_result = a.cmp(b);
            assert_eq!(
                partial_result,
                Some(full_result),
                "PartialOrd/Ord inconsistency for {:?} and {:?}: {:?} != {:?}",
                a.as_str(),
                b.as_str(),
                partial_result,
                full_result
            );

            // Test comparison consistency
            let str_cmp = a.as_str().cmp(b.as_str());
            let sinstr_cmp = a.cmp(b);
            assert_eq!(
                str_cmp,
                sinstr_cmp,
                "String comparison doesn't match SinStr comparison for {:?} and {:?}",
                a.as_str(),
                b.as_str()
            );

            // Test operator consistency
            let a_less_b = a < b;
            let a_greater_b = a > b;
            let a_leq_b = a <= b;
            let a_geq_b = a >= b;

            match a.cmp(b) {
                Ordering::Less => {
                    assert!(
                        a_less_b,
                        "Expected a < b when cmp is Less: {:?}, {:?}",
                        a.as_str(),
                        b.as_str()
                    );
                    assert!(!a_greater_b, "Expected !(a > b) when cmp is Less");
                    assert!(a_leq_b, "Expected a <= b when cmp is Less");
                    assert!(!a_geq_b, "Expected !(a >= b) when cmp is Less");
                }
                Ordering::Equal => {
                    assert!(!a_less_b, "Expected !(a < b) when cmp is Equal");
                    assert!(!a_greater_b, "Expected !(a > b) when cmp is Equal");
                    assert!(a_leq_b, "Expected a <= b when cmp is Equal");
                    assert!(a_geq_b, "Expected a >= b when cmp is Equal");
                }
                Ordering::Greater => {
                    assert!(!a_less_b, "Expected !(a < b) when cmp is Greater");
                    assert!(a_greater_b, "Expected a > b when cmp is Greater");
                    assert!(!a_leq_b, "Expected !(a <= b) when cmp is Greater");
                    assert!(a_geq_b, "Expected a >= b when cmp is Greater");
                }
            }
        }
    }

    // Test anti-symmetry: if a < b then !(b < a)
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            let a = &strings[i];
            let b = &strings[j];

            if a < b {
                assert!(
                    !(b < a),
                    "Anti-symmetry failed: {:?} < {:?} but also {:?} < {:?}",
                    a.as_str(),
                    b.as_str(),
                    b.as_str(),
                    a.as_str()
                );
                assert!(
                    b > a,
                    "Anti-symmetry via > failed: {:?} < {:?} but !(b > a)",
                    a.as_str(),
                    b.as_str()
                );
            }

            if a > b {
                assert!(
                    !(b > a),
                    "Anti-symmetry failed: {:?} > {:?} but also {:?} > {}",
                    a.as_str(),
                    b.as_str(),
                    b.as_str(),
                    a.as_str()
                );
                assert!(
                    b < a,
                    "Anti-symmetry via < failed: {:?} > {:?} but !(b < a)",
                    a.as_str(),
                    b.as_str()
                );
            }
        }
    }

    // Test transitivity: a < b && b < c implies a < c
    for i in 0..strings.len().min(20) {
        for j in 0..strings.len().min(20) {
            for k in 0..strings.len().min(20) {
                let a = &strings[i];
                let b = &strings[j];
                let c = &strings[k];

                if a < b && b < c {
                    assert!(
                        a < c,
                        "Transitivity failed: {:?} < {:?} and {:?} < {:?} but {:?} >= {:?}",
                        a.as_str(),
                        b.as_str(),
                        b.as_str(),
                        c.as_str(),
                        a.as_str(),
                        c.as_str()
                    );
                }

                if a > b && b > c {
                    assert!(
                        a > c,
                        "Transitivity failed: {:?} > {:?} and {:?} > {:?} but {:?} <= {:?}",
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

    // Test cross-storage comparison (inline vs heap)
    for i in 0..strings.len().min(10) {
        for j in 0..strings.len().min(10) {
            let a = &strings[i];
            let b = &strings[j];

            // Storage mode should not affect ordering - only content matters
            let str_cmp = a.as_str().cmp(b.as_str());
            let sinstr_cmp = a.cmp(b);

            // Ordering must be consistent with underlying string
            assert_eq!(str_cmp, sinstr_cmp, 
                       "Ordering inconsistent for {:?} ({}) vs {:?} ({}): string says {:?}, SinStr says {:?}",
                       a.as_str(), if a.is_heap() { "heap" } else { "inline" },
                       b.as_str(), if b.is_heap() { "heap" } else { "inline" },
                       str_cmp, sinstr_cmp);
        }
    }

    // Test total ordering: for all (a,b), exactly one of: a < b, a == b, a > b
    for i in 0..strings.len() {
        for j in 0..strings.len() {
            let a = &strings[i];
            let b = &strings[j];

            let less = a < b;
            let equal = a == b;
            let greater = a > b;

            // Exactly one must be true
            assert_eq!(
                less as u8 + equal as u8 + greater as u8,
                1,
                "Total ordering violated for {:?} and {:?}: <={}, =={}, >={}",
                a.as_str(),
                b.as_str(),
                less,
                equal,
                greater
            );
        }
    }
});
