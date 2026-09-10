//! The LCA reduction, which every peptide-to-taxon endpoint runs once per hit.
//!
//! `calculate_lca` looks a taxon up in two stores, then walks the ranks over the lineages it
//! collected. The walk stops at the first rank two lineages disagree at, so what the call costs is
//! set almost entirely by the lookups: by how many taxa a request carries, and by how many of them
//! are distinct, because a wide distinct set does not fit in cache.
//!
//! Every caller now hands over the distinct taxa of a hit, so the draws are deduplicated here too.
//! The dedup is inside the timed loop because the callers pay for it: it is a hash per draw, and
//! at this ratio of draws to distinct taxa it is about half of what the call costs.
//!
//! Both numbers therefore have to be realistic, which is what `fixtures::synthetic` is for. The
//! taxa it generates span two domains, so the reduction shares no rank, answers root and walks all
//! 28 of them. That is the worst case.

use std::hint::black_box;

use criterion::Criterion;
use itertools::Itertools;
use tempfile::TempDir;
use unipept_api::helpers::lca_helper::calculate_lca;

/// Distinct taxa, and the number of times they are drawn, at the scale of a large request.
const DISTINCT_TAXA: u32 = 26_919;
const DRAWS: usize = 463_423;

pub fn lca_benchmark(c: &mut Criterion) {
    // The stores hold everything they read, so the directory is only alive long enough to be
    // parsed out of.
    let dir = TempDir::new().expect("could not create a temporary directory");
    let (taxon_store, lineage_store) = fixtures::synthetic::load_taxonomy(dir.path(), DISTINCT_TAXA);
    let taxa = fixtures::synthetic::draws(DISTINCT_TAXA, DRAWS);

    // Checked, not assumed, and not a `debug_assert`: `[profile.bench]` leaves debug assertions
    // off, so one would never run. This costs a single call outside the timed loop.
    assert_eq!(
        calculate_lca(taxa.iter().copied().unique(), &taxon_store, &lineage_store, true),
        1,
        "the taxa must span two domains, so that every rank is walked"
    );

    c.bench_function("calculate_lca", |b| {
        b.iter(|| black_box(calculate_lca(taxa.iter().copied().unique(), &taxon_store, &lineage_store, true)))
    });
}
