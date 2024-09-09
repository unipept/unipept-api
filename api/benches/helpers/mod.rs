use criterion::criterion_group;

mod lca_helper;

criterion_group!(benches, lca_helper::lca_benchmark);