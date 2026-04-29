use criterion::criterion_group;

mod aggregation;

criterion_group!(benches, aggregation::aggregation_benchmarks);
