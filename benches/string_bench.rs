use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

const TEST_STRING_INLINE: &str = "test";
const TEST_STRING_HEAP: &str = "this is a much longer string that requires heap allocation!";

fn bench_new(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = c.benchmark_group(name);
    group.sample_size(1000);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("String", |b| {
        b.iter_with_large_drop(|| String::from(black_box(test_str)))
    });

    group.bench_function("Box<str>", |b| {
        b.iter_with_large_drop(|| Box::<str>::from(black_box(test_str)))
    });

    group.bench_function("CompactString", |b| {
        b.iter_with_large_drop(|| compact_str::CompactString::new(black_box(test_str)))
    });

    group.bench_function("SmolStr", |b| {
        b.iter_with_large_drop(|| smol_str::SmolStr::new(black_box(test_str)))
    });

    group.bench_function("SmartString<Compact>", |b| {
        b.iter_with_large_drop(|| {
            smartstring::SmartString::<smartstring::Compact>::from(black_box(test_str))
        })
    });

    group.bench_function("SmartString<LazyCompact>", |b| {
        b.iter_with_large_drop(|| {
            smartstring::SmartString::<smartstring::LazyCompact>::from(black_box(test_str))
        })
    });

    group.bench_function("SinStr", |b| {
        b.iter_with_large_drop(|| sinstr::SinStr::new(black_box(test_str)))
    });

    group.finish();
}

fn bench_as_str(c: &mut Criterion, name: &str, test_str: &'static str) {
    let mut group = c.benchmark_group(name);
    group.sample_size(1000);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    let string: String = String::from(test_str);
    let box_str: Box<str> = Box::from(test_str);
    let compact: compact_str::CompactString = compact_str::CompactString::new(test_str);
    let smolstr: smol_str::SmolStr = smol_str::SmolStr::new(test_str);
    let smartstring: smartstring::SmartString<smartstring::LazyCompact> =
        smartstring::SmartString::from(test_str);
    let sinstr: sinstr::SinStr = sinstr::SinStr::new(test_str);

    group.bench_function("String", |b| {
        b.iter(|| black_box(black_box(&string).as_str()))
    });

    group.bench_function("Box<str>", |b| {
        b.iter(|| black_box(black_box(&box_str).as_ref()))
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| black_box(black_box(&compact).as_str()))
    });

    group.bench_function("SmolStr", |b| {
        b.iter(|| black_box(black_box(&smolstr).as_str()))
    });

    group.bench_function("SmartString", |b| {
        b.iter(|| black_box(black_box(&smartstring).as_str()))
    });

    group.bench_function("SinStr", |b| {
        b.iter(|| black_box(black_box(&sinstr).as_str()))
    });

    group.finish();
}

fn bench_vec_iterate(c: &mut Criterion, name: &str, test_str: &'static str, count: usize) {
    let mut group = c.benchmark_group(name);
    group.sample_size(1000);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    let vec_string: Vec<String> = (0..count).map(|_| test_str.to_string()).collect();
    let vec_box_str: Vec<Box<str>> = (0..count).map(|_| Box::from(test_str)).collect();
    let vec_compact: Vec<compact_str::CompactString> = (0..count)
        .map(|_| compact_str::CompactString::new(test_str))
        .collect();
    let vec_smolstr: Vec<smol_str::SmolStr> = (0..count)
        .map(|_| smol_str::SmolStr::new(test_str))
        .collect();
    let vec_smartstring_lazy: Vec<smartstring::SmartString<smartstring::LazyCompact>> = (0..count)
        .map(|_| smartstring::SmartString::from(test_str))
        .collect();
    let vec_smartstring_compact: Vec<smartstring::SmartString<smartstring::LazyCompact>> = (0
        ..count)
        .map(|_| smartstring::SmartString::from(test_str))
        .collect();
    let vec_sinstr: Vec<sinstr::SinStr> =
        (0..count).map(|_| sinstr::SinStr::new(test_str)).collect();

    group.bench_function("String", |b| {
        b.iter(|| {
            for s in black_box(&vec_string) {
                black_box(s.as_str());
            }
        })
    });

    group.bench_function("Box<str>", |b| {
        b.iter(|| {
            for s in black_box(&vec_box_str) {
                black_box(s.as_ref());
            }
        })
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| {
            for s in black_box(&vec_compact) {
                black_box(s.as_str());
            }
        })
    });

    group.bench_function("SmolStr", |b| {
        b.iter(|| {
            for s in black_box(&vec_smolstr) {
                black_box(s.as_str());
            }
        })
    });

    group.bench_function("SmartString<Compact>", |b| {
        b.iter(|| {
            for s in black_box(&vec_smartstring_compact) {
                black_box(s.as_str());
            }
        })
    });

    group.bench_function("SmartString<Lazy>", |b| {
        b.iter(|| {
            for s in black_box(&vec_smartstring_lazy) {
                black_box(s.as_str());
            }
        })
    });

    group.bench_function("SinStr", |b| {
        b.iter(|| {
            for s in black_box(&vec_sinstr) {
                black_box(s.as_str());
            }
        })
    });

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

criterion_group!(
    benches,
    new_benchmarks,
    as_str_benchmarks,
    vec_iterate_benchmarks
);
criterion_main!(benches);
