use criterion::criterion_group;

mod fa_helper;
mod lca_helper;

criterion_group!(benches, lca_helper::lca_benchmark, fa_helper::fa_benchmark);
