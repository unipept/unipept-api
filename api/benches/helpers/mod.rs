use criterion::criterion_group;

mod fa_helper;
mod lca_helper;
mod taxa_filter;

criterion_group!(benches, lca_helper::lca_benchmark, fa_helper::fa_benchmark, taxa_filter::taxa_filter_benchmark);
