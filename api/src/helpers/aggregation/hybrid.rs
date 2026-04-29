use std::collections::HashMap;
use datastore::{LineageStore, TaxonStore};

use crate::helpers::lineage_helper::{get_amount_of_ranks, get_lineage_array_numeric, LineageVersion};

pub fn calculate_hybrid(
    taxa: Vec<u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore,
    only_valid_taxa: bool,
    threshold: f64,
) -> i32 {
    let mut lineages: Vec<Vec<i32>> = taxa
        .into_iter()
        .filter(|&id| !only_valid_taxa || taxon_store.is_valid(id))
        .map(|id| get_lineage_array_numeric(id, version, lineage_store))
        .collect();

    if lineages.is_empty() {
        return 1;
    }

    let n_ranks = get_amount_of_ranks(version) as usize;
    let mut counts: HashMap<i32, usize> = HashMap::with_capacity(lineages.len());
    let mut result = 1i32;

    for rank in 0..n_ranks {
        counts.clear();
        for lineage in &lineages {
            let v = lineage[rank];
            if v > 0 {
                *counts.entry(v).or_insert(0) += 1;
            }
        }

        if counts.is_empty() {
            continue;
        }
        let (&best_value, &best_count) = counts.iter().max_by_key(|(_, &c)| c).unwrap();

        if best_count as f64 / lineages.len() as f64 >= threshold {
            result = best_value;
            lineages.retain(|l| l[rank] == best_value);
        } else {
            break;
        }
    }

    result
}


#[cfg(test)]
mod tests {
    use datastore::{LineageStore, TaxonStore};
    use crate::helpers::aggregation::hybrid::calculate_hybrid;
    use crate::helpers::aggregation::lca::calculate_lca;
    use crate::helpers::lineage_helper::LineageVersion;

    fn load_stores() -> (TaxonStore, LineageStore) {
        let taxon_store = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");
        (taxon_store, lineage_store)
    }

    #[test]
    fn small_test_calculate_hybrid_f1() {
        // With f=1.0 only unanimous ranks are accepted, matching conservative LCA behavior
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let lca_result = calculate_lca(taxa.clone(), version, &taxon_store, &lineage_store, true);
        let hybrid_result = calculate_hybrid(taxa, version, &taxon_store, &lineage_store, true, 1.0);

        assert_eq!(hybrid_result, lca_result);
    }

    #[test]
    fn small_test_calculate_hybrid_f0() {
        // With f=0.0 we descend as far as possible; result is always a valid taxon
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_hybrid(taxa, version, &taxon_store, &lineage_store, true, 0.0);

        assert!(result > 1, "f=0.0 should descend past the root");
    }

    #[test]
    fn single_taxon_returns_itself() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_hybrid(vec![8501], version, &taxon_store, &lineage_store, true, 0.5);
        assert_eq!(result, 8501);
    }

    #[test]
    fn empty_after_filter_returns_root() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        // taxon 27 is invalid, so only_valid_taxa=true leaves an empty set
        let result = calculate_hybrid(vec![27], version, &taxon_store, &lineage_store, true, 0.5);
        assert_eq!(result, 1);
    }

    #[test]
    fn higher_f_produces_ancestor_of_lower_f() {
        // A higher threshold must yield a result that is at least as ancestral (closer to root)
        // as a lower threshold. We verify this by checking that the f=0.75 result appears in
        // the lineage of the f=0.25 result.
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let conservative = calculate_hybrid(taxa.clone(), version, &taxon_store, &lineage_store, true, 0.75);
        let aggressive   = calculate_hybrid(taxa,          version, &taxon_store, &lineage_store, true, 0.25);

        // conservative result must appear in (or equal) the lineage of aggressive result
        let aggressive_lineage = crate::helpers::lineage_helper::get_lineage_array_numeric(
            aggressive as u32, version, &lineage_store
        );
        assert!(
            conservative == aggressive
                || aggressive_lineage.contains(&conservative)
                || conservative == 1,
            "f=0.75 result {conservative} should be an ancestor of f=0.25 result {aggressive}"
        );
    }
}
