use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use unicode_truncate::UnicodeTruncateStr;

fn roughly_cut(str: &str, size: usize) -> &str {
    if size >= str.len() {
        return str;
    }
    let mut end = size;
    while !str.is_char_boundary(end) {
        end += 1;
    }
    &str[..end]
}

fn criterion_benchmark(criterion: &mut Criterion) {
    static KB: usize = 1024;
    static TEXT: &str = include_str!("data/zhufu.txt");

    let mut group = criterion.benchmark_group("zhu fu");
    group
        .sample_size(1000)
        .measurement_time(Duration::from_secs(20));
    for size in &[KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB, 28 * KB] {
        let str = roughly_cut(TEXT, *size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), str, |bench, str| {
            bench.iter(|| str.unicode_truncate(str.len() / 2));
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
