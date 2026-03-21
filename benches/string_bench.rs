use compact_str::CompactString;
use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use sinstr::SinStr;
use smartstring::{Compact, LazyCompact, SmartString};
use smol_str::SmolStr;
use std::time::Duration;

const TEST_STRING_INLINE: &str = "test";
const TEST_STRING_HEAP: &str = "this is a much longer string that requires heap allocation!";

fn setup_group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(1000);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    group
}

macro_rules! bench_new {
    ($name:expr, $group:expr, $create:expr) => {
        $group.bench_function($name, |b| b.iter_with_large_drop(|| black_box($create)));
    };
}

macro_rules! bench_clone {
    ($name:expr, $group:expr, $value:expr) => {
        $group.bench_function($name, |b| {
            b.iter_with_large_drop(|| black_box(black_box($value).clone()))
        });
    };
}

macro_rules! bench_eq {
    ($name:expr, $group:expr, $lhs:expr, $rhs:expr) => {
        $group.bench_function($name, |b| {
            b.iter(|| black_box(black_box($lhs) == black_box($rhs)))
        });
    };
}

macro_rules! bench_drop {
    ($name:expr, $group:expr, $create:expr) => {
        $group.bench_function($name, |b| {
            b.iter_batched(|| $create, std::mem::drop, BatchSize::LargeInput)
        });
    };
}

macro_rules! bench_vec_iterate {
    ($name:expr, $group:expr, $vec:expr, $method:ident) => {
        $group.bench_function($name, |b| {
            b.iter(|| {
                for s in black_box(&$vec) {
                    black_box(black_box(s).$method());
                }
            })
        });
    };
}

macro_rules! bench_as_str {
    ($name:expr, $group:expr, $value:expr, $method:ident) => {
        $group.bench_function($name, |b| {
            b.iter(|| black_box(black_box(&$value).$method()))
        });
    };
}

fn bench_new(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = setup_group(c, name);
    bench_new!("String", group, String::from(black_box(test_str)));
    bench_new!("Box<str>", group, Box::<str>::from(black_box(test_str)));
    bench_new!(
        "CompactString",
        group,
        CompactString::new(black_box(test_str))
    );
    bench_new!("SmolStr", group, SmolStr::new(black_box(test_str)));
    bench_new!(
        "SmartString<Compact>",
        group,
        SmartString::<Compact>::from(black_box(test_str))
    );
    bench_new!(
        "SmartString<LazyCompact>",
        group,
        SmartString::<LazyCompact>::from(black_box(test_str))
    );
    bench_new!("SinStr", group, SinStr::new(black_box(test_str)));
    group.finish();
}

fn bench_as_str(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = setup_group(c, name);
    let string: String = String::from(test_str);
    let box_str: Box<str> = Box::from(test_str);
    let compact = CompactString::new(test_str);
    let smolstr = SmolStr::new(test_str);
    let smartstring_lazy = SmartString::<LazyCompact>::from(test_str);
    let smartstring_compact = SmartString::<Compact>::from(test_str);
    let sinstr = SinStr::new(test_str);

    bench_as_str!("String", group, string, as_str);
    bench_as_str!("Box<str>", group, box_str, as_ref);
    bench_as_str!("CompactString", group, compact, as_str);
    bench_as_str!("SmolStr", group, smolstr, as_str);
    bench_as_str!("SmartString<LazyCompact>", group, smartstring_lazy, as_str);
    bench_as_str!("SmartString<Compact>", group, smartstring_compact, as_str);
    bench_as_str!("SinStr", group, sinstr, as_str);
    group.finish();
}

fn bench_vec_iterate(c: &mut Criterion, name: &str, test_str: &'static str, count: usize) {
    let mut group = setup_group(c, name);

    let vec_string: Vec<String> = (0..count).map(|_| test_str.to_string()).collect();
    let vec_box_str: Vec<Box<str>> = (0..count).map(|_| Box::from(test_str)).collect();
    let vec_compact: Vec<CompactString> =
        (0..count).map(|_| CompactString::new(test_str)).collect();
    let vec_smolstr: Vec<SmolStr> = (0..count).map(|_| SmolStr::new(test_str)).collect();
    let vec_smartstring_lazy: Vec<SmartString<LazyCompact>> =
        (0..count).map(|_| SmartString::from(test_str)).collect();
    let vec_smartstring_compact: Vec<SmartString<LazyCompact>> =
        (0..count).map(|_| SmartString::from(test_str)).collect();
    let vec_sinstr: Vec<SinStr> = (0..count).map(|_| SinStr::new(test_str)).collect();

    bench_vec_iterate!("String", group, vec_string, as_str);
    bench_vec_iterate!("Box<str>", group, vec_box_str, as_ref);
    bench_vec_iterate!("CompactString", group, vec_compact, as_str);
    bench_vec_iterate!("SmolStr", group, vec_smolstr, as_str);
    bench_vec_iterate!(
        "SmartString<Compact>",
        group,
        vec_smartstring_compact,
        as_str
    );
    bench_vec_iterate!("SmartString<Lazy>", group, vec_smartstring_lazy, as_str);
    bench_vec_iterate!("SinStr", group, vec_sinstr, as_str);

    group.finish();
}

fn bench_clone(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = setup_group(c, name);
    let string: String = String::from(test_str);
    let box_str: Box<str> = Box::from(test_str);
    let compact = CompactString::new(test_str);
    let smolstr = SmolStr::new(test_str);
    let smartstring_lazy = SmartString::<LazyCompact>::from(test_str);
    let smartstring_compact = SmartString::<Compact>::from(test_str);
    let sinstr = SinStr::new(test_str);

    bench_clone!("String", group, &string);
    bench_clone!("Box<str>", group, &box_str);
    bench_clone!("CompactString", group, &compact);
    bench_clone!("SmolStr", group, &smolstr);
    bench_clone!("SmartString<LazyCompact>", group, &smartstring_lazy);
    bench_clone!("SmartString<Compact>", group, &smartstring_compact);
    bench_clone!("SinStr", group, &sinstr);
    group.finish();
}

fn bench_eq(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = setup_group(c, name);
    let string1 = String::from(test_str);
    let string2 = String::from(test_str);
    let box_str1: Box<str> = Box::from(test_str);
    let box_str2: Box<str> = Box::from(test_str);
    let compact1 = CompactString::new(test_str);
    let compact2 = CompactString::new(test_str);
    let smolstr1 = SmolStr::new(test_str);
    let smolstr2 = SmolStr::new(test_str);
    let smartstring_lazy1 = SmartString::<LazyCompact>::from(test_str);
    let smartstring_lazy2 = SmartString::<LazyCompact>::from(test_str);
    let smartstring_compact1 = SmartString::<Compact>::from(test_str);
    let smartstring_compact2 = SmartString::<Compact>::from(test_str);
    let sinstr1 = SinStr::new(test_str);
    let sinstr2 = SinStr::new(test_str);

    bench_eq!("String", group, &string1, &string2);
    bench_eq!("Box<str>", group, &box_str1, &box_str2);
    bench_eq!("CompactString", group, &compact1, &compact2);
    bench_eq!("SmolStr", group, &smolstr1, &smolstr2);
    bench_eq!(
        "SmartString<LazyCompact>",
        group,
        &smartstring_lazy1,
        &smartstring_lazy2
    );
    bench_eq!(
        "SmartString<Compact>",
        group,
        &smartstring_compact1,
        &smartstring_compact2
    );
    bench_eq!("SinStr", group, &sinstr1, &sinstr2);
    group.finish();
}

fn bench_eq_cross(c: &mut Criterion) {
    let mut group = setup_group(c, "eq_cross");

    let inline1 = SinStr::new(TEST_STRING_INLINE);
    let inline2 = SinStr::new(TEST_STRING_INLINE);
    let heap1 = SinStr::new(TEST_STRING_HEAP);
    let heap2 = SinStr::new(TEST_STRING_HEAP);

    bench_eq!("inline/inline", group, &inline1, &inline2);
    bench_eq!("inline/heap", group, &inline1, &heap1);
    bench_eq!("heap/heap", group, &heap1, &heap2);
    group.finish();
}

fn bench_drop(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = setup_group(c, name);
    bench_drop!("String", group, String::from(black_box(test_str)));
    bench_drop!("Box<str>", group, Box::<str>::from(black_box(test_str)));
    bench_drop!(
        "CompactString",
        group,
        CompactString::new(black_box(test_str))
    );
    bench_drop!("SmolStr", group, SmolStr::new(black_box(test_str)));
    bench_drop!(
        "SmartString<Compact>",
        group,
        SmartString::<Compact>::from(black_box(test_str))
    );
    bench_drop!(
        "SmartString<LazyCompact>",
        group,
        SmartString::<LazyCompact>::from(black_box(test_str))
    );
    bench_drop!("SinStr", group, SinStr::new(black_box(test_str)));
    group.finish();
}

fn new_benchmarks(c: &mut Criterion) {
    bench_new(c, "new/inline", TEST_STRING_INLINE);
    bench_new(c, "new/heap", TEST_STRING_HEAP);
}

fn as_str_benchmarks(c: &mut Criterion) {
    bench_as_str(c, "as_str/inline", TEST_STRING_INLINE);
    bench_as_str(c, "as_str/heap", TEST_STRING_HEAP);
}

fn vec_iterate_benchmarks(c: &mut Criterion) {
    let sizes = [(32, 32), (64, 64), (128, 128), (256, 256)];

    for (width, height) in sizes {
        let count = width * height;
        bench_vec_iterate(
            c,
            &format!("vec_iterate_inline/{}x{}", width, height),
            TEST_STRING_INLINE,
            count,
        );
        bench_vec_iterate(
            c,
            &format!("vec_iterate_heap/{}x{}", width, height),
            TEST_STRING_HEAP,
            count,
        );
    }
}

fn clone_benchmarks(c: &mut Criterion) {
    bench_clone(c, "clone/inline", TEST_STRING_INLINE);
    bench_clone(c, "clone/heap", TEST_STRING_HEAP);
}

fn eq_benchmarks(c: &mut Criterion) {
    bench_eq(c, "eq/inline", TEST_STRING_INLINE);
    bench_eq(c, "eq/heap", TEST_STRING_HEAP);
    bench_eq_cross(c);
}

fn drop_benchmarks(c: &mut Criterion) {
    bench_drop(c, "drop/inline", TEST_STRING_INLINE);
    bench_drop(c, "drop/heap", TEST_STRING_HEAP);
}

criterion_group!(
    benches,
    new_benchmarks,
    as_str_benchmarks,
    vec_iterate_benchmarks,
    clone_benchmarks,
    eq_benchmarks,
    drop_benchmarks
);
criterion_main!(benches);
