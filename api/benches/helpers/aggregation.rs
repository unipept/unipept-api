use criterion::{black_box, BatchSize, BenchmarkGroup, Criterion};
use datastore::{LineageStore, TaxonStore};
use unipept_api::helpers::{
    aggregation::{hybrid::calculate_hybrid, iterative::calculate_iterative, lca::calculate_lca, lca_star::calculate_lca_star, mrtl::calculate_mrtl},
    lineage_helper::LineageVersion,
};
use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

fn read_taxa_file() -> Vec<u32> {
    let filename = "../data/taxa_from_400_peptides.txt";
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line").parse::<u32>().unwrap())
        .collect()
}

fn generate_arguments() -> (Vec<u32>, LineageVersion, TaxonStore, LineageStore) {
    let taxa = read_taxa_file();
    let version = LineageVersion::V2;
    let taxon_store = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
    let lineage_store = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");
    (taxa, version, taxon_store, lineage_store)
}

fn bench_lca(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    group.bench_function("lca", |b| {
        b.iter_batched(
            generate_arguments,
            |(taxa, version, taxon_store, lineage_store)| {
                black_box(calculate_lca(taxa, version, &taxon_store, &lineage_store, true))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_mrtl(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    group.bench_function("mrtl", |b| {
        b.iter_batched(
            generate_arguments,
            |(taxa, version, taxon_store, lineage_store)| {
                black_box(calculate_mrtl(taxa, version, &taxon_store, &lineage_store, true))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_lca_star(group: &mut BenchmarkGroup<criterion::measurement::WallTime>) {
    group.bench_function("lca_star", |b| {
        b.iter_batched(
            generate_arguments,
            |(taxa, version, taxon_store, lineage_store)| {
                black_box(calculate_lca_star(taxa, version, &taxon_store, &lineage_store, true))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_hybrid(group: &mut BenchmarkGroup<criterion::measurement::WallTime>, f: f64, name: &str) {
    group.bench_function(name, |b| {
        b.iter_batched(
            generate_arguments,
            |(taxa, version, taxon_store, lineage_store)| {
                black_box(calculate_hybrid(taxa, version, &taxon_store, &lineage_store, true, f))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_iterative(group: &mut BenchmarkGroup<criterion::measurement::WallTime>, threshold: f64, name: &str) {
    group.bench_function(name, |b| {
        b.iter_batched(
            generate_arguments,
            |(taxa, version, taxon_store, lineage_store)| {
                black_box(calculate_iterative(taxa, version, &taxon_store, &lineage_store, true, threshold))
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn aggregation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregation");

    bench_lca(&mut group);
    bench_lca_star(&mut group);
    bench_mrtl(&mut group);
    bench_hybrid(&mut group, 0.75, "hybrid_f75");
    bench_hybrid(&mut group, 0.50, "hybrid_f50");
    bench_hybrid(&mut group, 0.25, "hybrid_f25");
    bench_iterative(&mut group, 1.00, "iterative_t100");
    bench_iterative(&mut group, 0.50, "iterative_t50");

    group.finish();
}
