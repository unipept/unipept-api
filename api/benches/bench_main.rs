use criterion::criterion_main;

mod helpers;

criterion_main!(helpers::benches);