use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};

use unicode_truncate::UnicodeTruncateStr;
use std::time::Duration;

fn roughly_cut(s: &str, size: usize) -> &str {
    if size >= s.len() {
        return s;
    }
    let mut end = size;
    while !s.is_char_boundary(end) {
        end += 1;
    }
    &s[..end]
}

fn criterion_benchmark(c: &mut Criterion) {
    static KB: usize = 1024;
    static TEXT: &str = include_str!("data/zhufu.txt");

    let mut group = c.benchmark_group("zhu fu");
    group.sample_size(1000).measurement_time(Duration::from_secs(20));
    for size in [KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB, 28 * KB].iter() {
        let s = roughly_cut(TEXT, *size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), s, |b, s| {
            b.iter(|| s.unicode_truncate(s.len() / 2));
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
