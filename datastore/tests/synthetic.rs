//! The generated taxonomy, read through the two stores that parse it.
//!
//! `fixtures::synthetic` writes the taxonomy a benchmark measures over. It cannot check its own
//! output against the parsers, because `datastore` dev-depends on `fixtures` and the dependency
//! would cycle, so the round trip is asserted from this side.

use std::collections::BTreeSet;

use datastore::{LineageStore, TaxonStore};
use fixtures::synthetic::{self, LEAF_BASE};
use tempfile::TempDir;

const DISTINCT: u32 = 4_096;

/// The `TempDir` is dropped by the caller rather than here, so that a failure leaves the files it
/// read behind to look at.
fn load() -> (TempDir, TaxonStore, LineageStore) {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let (taxons, lineages) = synthetic::load_taxonomy(dir.path(), DISTINCT);

    (dir, taxons, lineages)
}

/// Both parsers accept the generated files, and every leaf is in both of them.
#[test]
fn every_leaf_has_a_taxon_row_and_a_lineage_row() {
    let (_dir, taxons, lineages) = load();

    for leaf in 0..DISTINCT {
        let id = LEAF_BASE + leaf;
        assert!(taxons.get(id).is_some(), "leaf {id} has no taxon row");
        assert!(taxons.is_valid(id), "leaf {id} must be valid");
        assert!(lineages.get(id).is_some(), "leaf {id} has no lineage row");
    }
}

/// Every ancestor a lineage names resolves through the taxon store.
///
/// This is what an LCA answer is: `calculate_lca` returns an ancestor id, and a caller then asks
/// the taxon store for its name and rank. A taxonomy of leaves alone parses and walks perfectly
/// well, and gives `None` for every answer it produces.
#[test]
fn every_ancestor_a_lineage_names_resolves() {
    let (_dir, taxons, lineages) = load();

    let mut checked = 0;
    for leaf in 0..DISTINCT {
        let lineage = lineages.get(LEAF_BASE + leaf).expect("every leaf has a lineage");

        for rank in 0..28 {
            let Some(ancestor) = lineage.get_rank(rank) else {
                continue;
            };

            let id = ancestor.unsigned_abs();
            assert!(taxons.get(id).is_some(), "ancestor {id}, at rank {rank} of leaf {leaf}, has no taxon row");
            checked += 1;
        }
    }

    assert!(checked > 0, "the lineages recorded no ancestors at all");
}

/// Root resolves too. It is what the reduction answers when the taxa share nothing, which is the
/// case this taxonomy is built to produce.
#[test]
fn the_root_taxon_resolves() {
    let (_dir, taxons, _lineages) = load();

    let (name, _, _) = taxons.get(1).expect("root has a taxon row");
    assert_eq!(name, "root");
}

/// The two domains reach the store as two distinct taxa, so a reduction over the whole set shares
/// no rank and runs to root.
#[test]
fn the_leaves_span_two_domains() {
    let (_dir, _taxons, lineages) = load();

    let domains: BTreeSet<i32> = (0..DISTINCT)
        .filter_map(|leaf| lineages.get(LEAF_BASE + leaf).expect("every leaf has a lineage").get_rank(0))
        .collect();

    assert_eq!(domains.len(), 2);
}
