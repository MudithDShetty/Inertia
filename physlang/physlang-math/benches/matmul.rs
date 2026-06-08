use criterion::{black_box, criterion_group, criterion_main, Criterion};
use physlang_math::{bench_matmul, matmul, Tensor};

fn matmul_benchmark(c: &mut Criterion) {
    let n = 256;
    let a = Tensor::from_vec(
        vec![n, n],
        (0..n * n).map(|i| (i % 17) as f64 * 0.01).collect(),
    )
    .unwrap();
    let b = Tensor::from_vec(
        vec![n, n],
        (0..n * n).map(|i| (i % 13) as f64 * 0.02).collect(),
    )
    .unwrap();

    c.bench_function("matmul_256", |bench| {
        bench.iter(|| matmul(black_box(&a), black_box(&b)).unwrap());
    });

    c.bench_function("bench_matmul_helper_1000", |bench| {
        bench.iter(|| black_box(bench_matmul(1000, 1)));
    });
}

criterion_group!(benches, matmul_benchmark);
criterion_main!(benches);
