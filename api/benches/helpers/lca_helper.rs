use datastore::{LineageStore, TaxonStore};
use unipept_api::helpers::{lca_helper::calculate_lca, lineage_helper::LineageVersion};
use criterion::black_box;

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
    let taxa: Vec<u32> = read_taxa_file();
    let version: LineageVersion = LineageVersion::V2;
    let taxon_store: TaxonStore = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
    let lineage_store: LineageStore = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

    (taxa, version, taxon_store, lineage_store)
}

pub fn lca_benchmark(c: &mut criterion::Criterion) {
    c.bench_function("calculate_lca", |b| {
        b.iter_batched(
            generate_arguments,
            |arguments|  {
                let (taxa, version, taxon_store, lineage_store) = arguments;
                black_box(calculate_lca(taxa, version, &taxon_store, &lineage_store, true))
            },
            criterion::BatchSize::SmallInput
        )
    });
}
