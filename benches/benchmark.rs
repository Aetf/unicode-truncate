use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

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
    const KB: usize = 1024;
    // see data/README.md for where these come from
    const TEXTS: &[(&str, &str)] = &[
        ("zhu fu", include_str!("data/zhufu.txt")),
        ("alice", include_str!("data/alice.txt")),
        ("kalila", include_str!("data/kalila.txt")),
        ("emoji", include_str!("data/emoji.txt")),
    ];

    for &(name, text) in TEXTS {
        for &size in &[KB, 4 * KB, 16 * KB, 28 * KB] {
            let input = roughly_cut(text, size);
            // how much is kept changes how much work truncation has to do, so cover both a
            // terminal line worth of columns and half of the input
            let widths = [("narrow", 80), ("half", input.width() / 2)];
            for (regime, max_width) in widths {
                let mut group = criterion.benchmark_group(format!("{name}/{size}/{regime}"));
                group
                    .sample_size(200)
                    .warm_up_time(Duration::from_secs(1))
                    .measurement_time(Duration::from_secs(5))
                    .throughput(Throughput::Bytes(size as u64));
                group.bench_function("end", |bench| {
                    bench.iter(|| black_box(input).unicode_truncate(black_box(max_width)));
                });
                group.bench_function("start", |bench| {
                    bench.iter(|| black_box(input).unicode_truncate_start(black_box(max_width)));
                });
                group.bench_function("centered", |bench| {
                    bench.iter(|| black_box(input).unicode_truncate_centered(black_box(max_width)));
                });
                group.finish();
            }
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
