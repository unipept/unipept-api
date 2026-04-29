use std::collections::HashSet;
use datastore::{LineageStore, TaxonStore};

use crate::helpers::aggregation::lca::calculate_lca;
use crate::helpers::aggregation::TaxaAggregation;
use crate::helpers::lineage_helper::{get_lineage_array_numeric, LineageVersion};

pub struct LcaStar;

impl TaxaAggregation for LcaStar {
    fn aggregate(&self, taxa: Vec<u32>, version: LineageVersion, taxon_store: &TaxonStore, lineage_store: &LineageStore, only_valid_taxa: bool) -> i32 {
        calculate_lca_star(taxa, version, taxon_store, lineage_store, only_valid_taxa)
    }
}

pub fn calculate_lca_star(
    taxa: Vec<u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore,
    only_valid_taxa: bool,
) -> i32 {
    let cleaned: Vec<u32> = taxa
        .into_iter()
        .filter(|&id| !only_valid_taxa || taxon_store.is_valid(id))
        .collect();

    if cleaned.is_empty() {
        return 1;
    }

    let taxon_set: HashSet<u32> = cleaned.iter().copied().collect();

    let mut ancestors_to_remove: HashSet<i32> = HashSet::new();
    for &id in &cleaned {
        for v in get_lineage_array_numeric(id, version, lineage_store) {
            if v > 0 && v as u32 != id && taxon_set.contains(&(v as u32)) {
                ancestors_to_remove.insert(v);
            }
        }
    }

    let pruned: Vec<u32> = cleaned
        .into_iter()
        .filter(|&id| !ancestors_to_remove.contains(&(id as i32)))
        .collect();

    calculate_lca(pruned, version, taxon_store, lineage_store, false)
}


#[cfg(test)]
mod tests {
    use datastore::{LineageStore, TaxonStore};
    use crate::helpers::aggregation::lca::calculate_lca;
    use crate::helpers::aggregation::lca_star::calculate_lca_star;
    use crate::helpers::lineage_helper::LineageVersion;

    fn load_stores() -> (TaxonStore, LineageStore) {
        let taxon_store = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");
        (taxon_store, lineage_store)
    }

    #[test]
    fn small_test_lca_star_with_ancestor_in_list() {
        // 8287 is an ancestor of both 8501 and 8505; LCA* should discard 8287 and return a
        // deeper result than standard LCA would on the same input
        let taxa_with_ancestor: Vec<u32> = vec![8287, 8501, 8505];
        let taxa_without_ancestor: Vec<u32> = vec![8501, 8505];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let lca_star_result = calculate_lca_star(taxa_with_ancestor, version, &taxon_store, &lineage_store, true);
        let expected = calculate_lca(taxa_without_ancestor, version, &taxon_store, &lineage_store, true);

        assert_eq!(lca_star_result, expected);
    }

    #[test]
    fn lca_star_without_ancestors_matches_lca() {
        // When no input taxon is an ancestor of another, LCA* behaves identically to LCA
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let lca_result = calculate_lca(taxa.clone(), version, &taxon_store, &lineage_store, true);
        let lca_star_result = calculate_lca_star(taxa, version, &taxon_store, &lineage_store, true);

        assert_eq!(lca_star_result, lca_result);
    }

    #[test]
    fn single_taxon_returns_itself() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_lca_star(vec![8501], version, &taxon_store, &lineage_store, true);
        assert_eq!(result, 8501);
    }

    #[test]
    fn empty_after_filter_returns_root() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        // taxon 27 is invalid, so only_valid_taxa=true leaves an empty set
        let result = calculate_lca_star(vec![27], version, &taxon_store, &lineage_store, true);
        assert_eq!(result, 1);
    }

    #[test]
    fn ancestor_is_removed_before_lca() {
        // With 8287 (an ancestor) in the list, LCA* must not return 8287 itself — the pruning
        // should have discarded it before the LCA computation
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_lca_star(vec![8287, 8501, 8505], version, &taxon_store, &lineage_store, true);
        assert_ne!(result, 8287 as i32, "ancestor 8287 should have been pruned");
    }
}
