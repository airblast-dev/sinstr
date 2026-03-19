use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

// The 4-byte test string
const TEST_STRING: &str = "test";

fn as_str_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("as_str");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("String", |b| {
        b.iter(|| {
            let s = String::from(black_box(TEST_STRING));
            black_box(black_box(s).as_str());
            // Drop happens here
        })
    });

    group.bench_function("Box<str>", |b| {
        b.iter(|| {
            let s: Box<str> = Box::from(black_box(TEST_STRING));
            black_box(black_box(s).as_ref());
            // Drop happens here
        })
    });

    group.bench_function("CompactString", |b| {
        b.iter(|| {
            let s = compact_str::CompactString::new(black_box(TEST_STRING));
            black_box(black_box(s).as_str());
            // Drop happens here
        })
    });

    group.bench_function("SinStr", |b| {
        b.iter(|| {
            let s = sinstr::SinStr::new(black_box(TEST_STRING));
            black_box(black_box(s).as_str());
            // Drop happens here
        })
    });

    group.finish();
}

fn vec_iterate_benchmarks(c: &mut Criterion) {
    let sizes = [(32, 32), (64, 64), (128, 128), (256, 256)];

    for (width, height) in sizes {
        let mut group = c.benchmark_group(format!("vec_iterate/{}x{}", width, height));
        group.sample_size(100);
        group.measurement_time(Duration::from_secs(5));

        // Create test string of the specified length
        let test_str = "test".to_string();

        let count = width * height;
        // Pre-create Vecs with all string types
        let vec_string: Vec<String> = (0..count).map(|_| test_str.clone()).collect();
        let vec_box_str: Vec<Box<str>> = (0..count).map(|_| Box::from(test_str.as_str())).collect();
        let vec_compact: Vec<compact_str::CompactString> = (0..count)
            .map(|_| compact_str::CompactString::new(&test_str))
            .collect();
        let vec_sinstr: Vec<sinstr::SinStr> =
            (0..count).map(|_| sinstr::SinStr::new(&test_str)).collect();

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

        group.bench_function("SinStr", |b| {
            b.iter(|| {
                for s in black_box(&vec_sinstr) {
                    black_box(s.as_str());
                }
            })
        });

        group.finish();
    }
}

criterion_group!(benches, as_str_benchmarks, vec_iterate_benchmarks);
criterion_main!(benches);
