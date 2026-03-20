use sinstr::{SinStr, discriminant::NICHE_MAX_INT};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[inline(always)]
fn next_id() -> usize {
    COUNTER.fetch_xor(47, Ordering::SeqCst)
}

#[inline(always)]
fn inline_string() -> String {
    let rep = next_id() % NICHE_MAX_INT;
    rep.to_string().repeat(rep)
}

#[inline(always)]
fn heap_string() -> String {
    let rep = next_id() % 1024 + NICHE_MAX_INT + 1;
    rep.to_string().repeat(rep)
}

#[test]
fn test_sinstr_in_struct() {
    struct StringHolder {
        short: SinStr,
        long: SinStr,
    }

    let holder = StringHolder {
        short: SinStr::new("abc"),
        long: SinStr::new(&"x".repeat(NICHE_MAX_INT + 10)),
    };

    assert_eq!(holder.short.as_str(), "abc");
    assert_eq!(holder.long.len(), NICHE_MAX_INT + 10);
    assert!(holder.short.is_inlined());
    assert!(holder.long.is_heap());
}

#[test]
fn test_sinstr_in_vec() {
    #[allow(clippy::useless_vec)]
    let vec = vec![
        SinStr::new(""),
        SinStr::new("a"),
        SinStr::new(&"x".repeat(NICHE_MAX_INT)),
        SinStr::new(&"y".repeat(NICHE_MAX_INT + 1)),
    ];

    assert_eq!(vec.len(), 4);
    assert!(vec[0].is_empty());
    assert_eq!(vec[1].as_str(), "a");
    assert!(vec[2].is_inlined());
    assert!(vec[3].is_heap());

    for s in vec.iter() {
        assert_eq!(s.len(), s.as_str().len());
    }
}

#[test]
fn test_sinstr_assignment() {
    let mut s1 = SinStr::new("initial");
    assert_eq!(s1.as_str(), "initial");

    s1 = SinStr::new("x");
    assert_eq!(s1.as_str(), "x");
    assert!(s1.is_inlined());

    s1 = SinStr::new(&"y".repeat(NICHE_MAX_INT + 1));
    assert!(s1.is_heap());

    s1 = SinStr::new("");
    assert!(s1.is_empty());
}

mod vec_tests {
    use super::*;

    #[test]
    fn test_vec_push_mixed_storage() {
        #[allow(clippy::useless_vec)]
        let vec = vec![
            SinStr::new("a"),
            SinStr::new(&heap_string()),
            SinStr::new(""),
            SinStr::new(&inline_string()),
        ];

        assert_eq!(vec.len(), 4);
        assert!(vec[0].is_inlined());
        assert!(vec[1].is_heap());
        assert!(vec[2].is_empty());
        assert!(vec[3].is_inlined());
    }

    #[test]
    fn test_vec_pop() {
        let mut vec: Vec<SinStr> = vec![
            SinStr::new("inline"),
            SinStr::new(&heap_string()),
            SinStr::new("end"),
        ];

        let last = vec.pop().unwrap();
        assert_eq!(last.as_str(), "end");
        assert!(last.is_inlined());

        let second_last = vec.pop().unwrap();
        assert!(second_last.is_heap());

        let first = vec.pop().unwrap();
        assert_eq!(first.as_str(), "inline");

        assert!(vec.is_empty());
    }

    #[test]
    fn test_vec_sort() {
        let mut vec: Vec<SinStr> = vec![
            SinStr::new("zebra"),
            SinStr::new(&format!("heap_a_{}", "x".repeat(NICHE_MAX_INT + 5))),
            SinStr::new("apple"),
            SinStr::new("banana"),
            SinStr::new(&format!("heap_b_{}", "y".repeat(NICHE_MAX_INT + 5))),
        ];

        vec.sort();

        let sorted: Vec<_> = vec.iter().map(|s| s.as_str()).collect();
        assert!(sorted.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn test_vec_dedup() {
        let mut vec: Vec<SinStr> = vec![
            SinStr::new("dup"),
            SinStr::new("dup"),
            SinStr::new("unique"),
            SinStr::new("unique"),
            SinStr::new("unique"),
        ];

        vec.dedup();

        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0].as_str(), "dup");
        assert_eq!(vec[1].as_str(), "unique");
    }

    #[test]
    fn test_vec_from_iterator() {
        let heap = heap_string();
        let strings = vec!["a", "bc", "def", heap.as_str()];
        let sinstr_vec: Vec<SinStr> = strings.into_iter().map(SinStr::new).collect();

        assert_eq!(sinstr_vec.len(), 4);
        assert!(sinstr_vec[0].is_inlined());
        assert!(sinstr_vec[1].is_inlined());
        assert!(sinstr_vec[2].is_inlined());
        assert!(sinstr_vec[3].is_heap());
    }

    #[test]
    fn test_vec_retain() {
        let mut vec: Vec<SinStr> = (0..10)
            .map(|i| SinStr::new(&format!("item{}", i)))
            .collect();

        vec.retain(|s| s.as_str().contains('5') || s.as_str().contains('7'));

        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0].as_str(), "item5");
        assert_eq!(vec[1].as_str(), "item7");
    }
}

mod hashmap_tests {
    use super::*;

    #[test]
    fn test_hashmap_insert_mixed() {
        let mut map = HashMap::new();

        map.insert(SinStr::new("inline_key"), SinStr::new("inline_value"));
        map.insert(SinStr::new(&heap_string()), SinStr::new(&heap_string()));
        map.insert(SinStr::new("mixed"), SinStr::new(&heap_string()));

        assert_eq!(map.len(), 3);
        assert!(map.contains_key(&SinStr::new("inline_key")));
        assert!(map.contains_key(&SinStr::new("mixed")));
    }

    #[test]
    fn test_hashmap_get() {
        let mut map = HashMap::new();
        let heap_key = SinStr::new(&heap_string());
        let heap_value = SinStr::new("heap_value");

        map.insert(SinStr::new("inline"), SinStr::new("val"));
        map.insert(heap_key.clone(), heap_value.clone());

        assert_eq!(
            map.get(&SinStr::new("inline")).map(|s| s.as_str()),
            Some("val")
        );
        assert_eq!(map.get(&heap_key).map(|s| s.as_str()), Some("heap_value"));
    }

    #[test]
    fn test_hashmap_entry_api() {
        let mut map = HashMap::new();

        map.entry(SinStr::new("key"))
            .or_insert(SinStr::new("default"));
        assert_eq!(map.get(&SinStr::new("key")).unwrap().as_str(), "default");

        map.entry(SinStr::new("key"))
            .or_insert(SinStr::new("ignored"));
        assert_eq!(map.get(&SinStr::new("key")).unwrap().as_str(), "default");

        map.entry(SinStr::new("key"))
            .and_modify(|v| *v = SinStr::new("modified"))
            .or_insert(SinStr::new("new"));
        assert_eq!(map.get(&SinStr::new("key")).unwrap().as_str(), "modified");
    }

    #[test]
    fn test_hashmap_remove() {
        let mut map = HashMap::new();
        let key = SinStr::new(&heap_string());

        map.insert(key.clone(), SinStr::new("value"));
        assert_eq!(map.len(), 1);

        let removed = map.remove(&key);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().as_str(), "value");
        assert!(map.is_empty());
    }

    #[test]
    fn test_hashmap_iteration() {
        let mut map = HashMap::new();
        map.insert(SinStr::new("k1"), SinStr::new("v1"));
        map.insert(SinStr::new("k2"), SinStr::new("v2"));
        map.insert(SinStr::new(&heap_string()), SinStr::new("v3"));

        let keys: Vec<_> = map.keys().collect();
        let values: Vec<_> = map.values().collect();
        let entries: Vec<_> = map.iter().collect();

        assert_eq!(keys.len(), 3);
        assert_eq!(values.len(), 3);
        assert_eq!(entries.len(), 3);
    }
}

mod btreemap_tests {
    use super::*;

    #[test]
    fn test_btreemap_insert_mixed() {
        let mut map = BTreeMap::new();

        map.insert(SinStr::new("a"), SinStr::new("first"));
        map.insert(SinStr::new("b"), SinStr::new("second"));
        map.insert(SinStr::new(&heap_string()), SinStr::new("heap"));

        assert_eq!(map.len(), 3);
        assert!(map.contains_key(&SinStr::new("a")));
    }

    #[test]
    fn test_btreemap_sorted_order() {
        let mut map = BTreeMap::new();

        map.insert(SinStr::new("z"), SinStr::new("last"));
        map.insert(SinStr::new("a"), SinStr::new("first"));
        map.insert(SinStr::new("m"), SinStr::new("middle"));
        map.insert(SinStr::new(&heap_string()), SinStr::new("heap"));

        let keys: Vec<_> = map.keys().collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn test_btreemap_range_query() {
        let mut map = BTreeMap::new();

        map.insert(SinStr::new("a"), SinStr::new("1"));
        map.insert(SinStr::new("c"), SinStr::new("3"));
        map.insert(SinStr::new("e"), SinStr::new("5"));
        map.insert(SinStr::new("g"), SinStr::new("7"));

        let range: Vec<_> = map.range(SinStr::new("b")..SinStr::new("f")).collect();

        assert_eq!(range.len(), 2);
        assert_eq!(range[0].0.as_str(), "c");
        assert_eq!(range[1].0.as_str(), "e");
    }

    #[test]
    fn test_btreemap_split_off() {
        let mut map = BTreeMap::new();

        map.insert(SinStr::new("a"), SinStr::new("1"));
        map.insert(SinStr::new("b"), SinStr::new("2"));
        map.insert(SinStr::new("c"), SinStr::new("3"));
        map.insert(SinStr::new("d"), SinStr::new("4"));

        let right = map.split_off(&SinStr::new("c"));

        assert_eq!(map.len(), 2);
        assert_eq!(right.len(), 2);
        assert!(map.contains_key(&SinStr::new("a")));
        assert!(right.contains_key(&SinStr::new("c")));
    }
}

mod hashset_tests {
    use super::*;

    #[test]
    fn test_hashset_insert_mixed() {
        let mut set = HashSet::new();

        assert!(set.insert(SinStr::new("inline")));
        assert!(set.insert(SinStr::new(&heap_string())));
        assert!(!set.insert(SinStr::new("inline")));

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_hashset_contains() {
        let mut set = HashSet::new();
        let heap_str = SinStr::new(&heap_string());

        set.insert(SinStr::new("inline"));
        set.insert(heap_str.clone());

        assert!(set.contains(&SinStr::new("inline")));
        assert!(set.contains(&heap_str));
        assert!(!set.contains(&SinStr::new("missing")));
    }

    #[test]
    fn test_hashset_union() {
        let mut set1 = HashSet::new();
        set1.insert(SinStr::new("a"));
        set1.insert(SinStr::new("b"));

        let mut set2 = HashSet::new();
        set2.insert(SinStr::new("b"));
        set2.insert(SinStr::new("c"));

        let union: HashSet<_> = set1.union(&set2).cloned().collect();

        assert_eq!(union.len(), 3);
        assert!(union.contains(&SinStr::new("a")));
        assert!(union.contains(&SinStr::new("b")));
        assert!(union.contains(&SinStr::new("c")));
    }

    #[test]
    fn test_hashset_intersection() {
        let mut set1 = HashSet::new();
        set1.insert(SinStr::new("a"));
        set1.insert(SinStr::new("b"));
        set1.insert(SinStr::new(&heap_string()));

        let mut set2 = HashSet::new();
        set2.insert(SinStr::new("b"));
        set2.insert(SinStr::new("c"));

        let intersection: HashSet<_> = set1.intersection(&set2).cloned().collect();

        assert_eq!(intersection.len(), 1);
        assert!(intersection.contains(&SinStr::new("b")));
    }

    #[test]
    fn test_hashset_difference() {
        let mut set1 = HashSet::new();
        set1.insert(SinStr::new("a"));
        set1.insert(SinStr::new("b"));

        let mut set2 = HashSet::new();
        set2.insert(SinStr::new("b"));
        set2.insert(SinStr::new("c"));

        let diff: HashSet<_> = set1.difference(&set2).cloned().collect();

        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&SinStr::new("a")));
    }

    #[test]
    fn test_hashset_drain() {
        let mut set = HashSet::new();
        set.insert(SinStr::new("a"));
        set.insert(SinStr::new("b"));
        set.insert(SinStr::new(&heap_string()));

        let drained: Vec<_> = set.drain().collect();

        assert!(set.is_empty());
        assert_eq!(drained.len(), 3);
    }
}

mod complex_scenarios {
    use super::*;

    #[test]
    fn test_collections_with_random_strings() {
        let mut vec = Vec::new();
        let mut hmap = HashMap::new();
        let mut bmap = BTreeMap::new();
        let mut hset = HashSet::new();

        for i in 0..100 {
            let key = if i % 2 == 0 {
                SinStr::new(&format!("inline_{}", i))
            } else {
                SinStr::new(&format!("{}_{}", "x".repeat(NICHE_MAX_INT + i), i))
            };

            vec.push(key.clone());
            hmap.insert(key.clone(), key.clone());
            bmap.insert(key.clone(), key.clone());
            hset.insert(key);
        }

        assert_eq!(vec.len(), 100);
        assert_eq!(hmap.len(), 100);
        assert_eq!(bmap.len(), 100);
        assert_eq!(hset.len(), 100);

        for v in &vec {
            assert!(hmap.contains_key(v));
            assert!(bmap.contains_key(v));
            assert!(hset.contains(v));
        }
    }

    #[test]
    fn test_collection_preserves_content() {
        let original_strings: Vec<String> = (0..50)
            .map(|i| {
                if i % 2 == 0 {
                    format!("inline{}", i)
                } else {
                    format!("{}_{}", "x".repeat(NICHE_MAX_INT + i), i)
                }
            })
            .collect();

        let sinstrs: Vec<SinStr> = original_strings.iter().map(|s| SinStr::new(s)).collect();

        for (original, sinstr) in original_strings.iter().zip(&sinstrs) {
            assert_eq!(sinstr.as_str(), original);
        }

        let map: HashMap<SinStr, SinStr> = sinstrs.iter().map(|s| (s.clone(), s.clone())).collect();

        for original in &original_strings {
            let key = SinStr::new(original);
            assert_eq!(map.get(&key).unwrap().as_str(), original);
        }
    }

    #[test]
    fn test_clone_collections() {
        let mut original_map: HashMap<SinStr, SinStr> = HashMap::new();

        for i in 0..10 {
            let key = if i % 3 == 0 {
                SinStr::new(&heap_string())
            } else {
                SinStr::new(&format!("key{}", i))
            };
            original_map.insert(key.clone(), key);
        }

        let cloned_map = original_map.clone();

        assert_eq!(original_map.len(), cloned_map.len());

        for (key, value) in &original_map {
            let cloned_value = cloned_map.get(key).unwrap();
            assert_eq!(value.as_str(), cloned_value.as_str());
        }

        for key in original_map.keys() {
            assert!(cloned_map.contains_key(key));
        }
    }

    #[test]
    fn test_vec_with_different_storage_transitions() {
        #[allow(clippy::useless_vec)]
        let mut vec = vec![SinStr::new("inline")];
        assert!(vec[0].is_inlined());

        vec[0] = SinStr::new(&heap_string());
        assert!(vec[0].is_heap());

        vec[0] = SinStr::new("");
        assert!(vec[0].is_empty());
    }

    #[test]
    fn test_nested_collections() {
        let mut outer: HashMap<SinStr, Vec<SinStr>> = HashMap::new();

        let key1 = SinStr::new("list1");
        let items1 = vec![
            SinStr::new("a"),
            SinStr::new(&heap_string()),
            SinStr::new("c"),
        ];

        let key2 = SinStr::new(&heap_string());
        let items2 = vec![SinStr::new(&heap_string()), SinStr::new("z")];

        outer.insert(key1.clone(), items1.clone());
        outer.insert(key2.clone(), items2.clone());

        assert_eq!(outer.get(&key1).unwrap().len(), 3);
        assert_eq!(outer.get(&key2).unwrap().len(), 2);

        let flattened: Vec<_> = outer.values().flatten().collect();
        assert_eq!(flattened.len(), 5);
    }
}
