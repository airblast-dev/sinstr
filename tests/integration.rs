use sinstr::NICHE_MAX_INT;
use sinstr::SinStr;

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
    let vec = [
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
