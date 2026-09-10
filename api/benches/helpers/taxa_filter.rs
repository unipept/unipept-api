//! The taxon filter, which `mpa/pept2data` runs once per protein hit.
//!
//! `filter` scans a taxon's ancestors for one the request named. Most of what it costs is the
//! `HashSet` probe it does per rank: up to `RANK_COUNT` SipHash-1-3 hashes of a `u32` per protein.
//! The `Vec` that used to hold a copy of the store's rank array is the smaller part — one
//! allocation per protein, about a tenth of the call, for a reader that keeps nothing.
//!
//! So this measures the hashing as much as the allocation. Anyone cutting the per-rank probe has
//! the larger target of the two.
//!
//! The draw is the case that costs the most: a filter that narrows to a clade the hits are not in,
//! so every rank is looked at and none matches. A filter that matches on the first rank returns
//! there and never reaches the rest.

use std::{collections::HashSet, hint::black_box};

use criterion::Criterion;
use index::ProteinInfo;
use tempfile::TempDir;
use unipept_api::helpers::filters::{UniprotFilter, taxa_filter::TaxaFilter};

/// Distinct taxa, and the number of protein hits scanned, at the scale of a large request.
const DISTINCT_TAXA: u32 = 26_919;
const PROTEINS: usize = 200_000;

/// A taxon no generated lineage names, so no protein passes the filter.
const UNREACHED_TAXON: u32 = 2;

pub fn taxa_filter_benchmark(c: &mut Criterion) {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let (_, lineage_store) = fixtures::synthetic::load_taxonomy(dir.path(), DISTINCT_TAXA);

    let proteins: Vec<ProteinInfo> = fixtures::synthetic::draws(DISTINCT_TAXA, PROTEINS)
        .into_iter()
        .map(|taxon| ProteinInfo { taxon, uniprot_accession: "P00001", annotations: &[] })
        .collect();

    let filter = TaxaFilter::new(HashSet::from([UNREACHED_TAXON]), &lineage_store);

    // Checked, not assumed, and not a `debug_assert`: `[profile.bench]` leaves debug assertions
    // off, so one would never run.
    assert!(
        !proteins.iter().any(|protein| filter.filter(protein)),
        "no protein may pass, so that every rank is scanned on every call"
    );

    c.bench_function("taxa_filter", |b| {
        b.iter(|| proteins.iter().filter(|protein| black_box(filter.filter(protein))).count())
    });
}
