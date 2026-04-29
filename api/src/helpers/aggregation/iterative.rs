use std::collections::HashMap;
use datastore::{LineageStore, TaxonStore};

use crate::helpers::aggregation::TaxaAggregation;
use crate::helpers::lineage_helper::{get_amount_of_ranks, get_lineage_array_numeric, LineageVersion};

pub struct Iterative { pub threshold: f64 }

impl TaxaAggregation for Iterative {
    fn aggregate(&self, taxa: Vec<u32>, version: LineageVersion, taxon_store: &TaxonStore, lineage_store: &LineageStore, only_valid_taxa: bool) -> i32 {
        calculate_iterative(taxa, version, taxon_store, lineage_store, only_valid_taxa, self.threshold)
    }
}

pub fn calculate_iterative(
    taxa: Vec<u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore,
    only_valid_taxa: bool,
    threshold: f64,
) -> i32 {
    let lineages: Vec<Vec<i32>> = taxa
        .into_iter()
        .filter(|&id| !only_valid_taxa || taxon_store.is_valid(id))
        .map(|id| get_lineage_array_numeric(id, version, lineage_store))
        .collect();

    if lineages.is_empty() {
        return 1;
    }

    // Denominator is fixed: penalises ranks where some proteins have no annotation.
    let total = lineages.len();
    let n_ranks = get_amount_of_ranks(version) as usize;
    let mut counts: HashMap<i32, usize> = HashMap::with_capacity(lineages.len());

    for rank in (0..n_ranks).rev() {
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

        if best_count as f64 / total as f64 >= threshold {
            return best_value;
        }
    }

    1
}


#[cfg(test)]
mod tests {
    use datastore::{LineageStore, TaxonStore};
    use crate::helpers::aggregation::iterative::calculate_iterative;
    use crate::helpers::lineage_helper::LineageVersion;

    fn load_stores() -> (TaxonStore, LineageStore) {
        let taxon_store = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");
        (taxon_store, lineage_store)
    }

    #[test]
    fn single_taxon_returns_itself() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_iterative(vec![8501], version, &taxon_store, &lineage_store, true, 1.0);
        assert_eq!(result, 8501);
    }

    #[test]
    fn empty_after_filter_returns_root() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        // taxon 27 is invalid, so only_valid_taxa=true leaves an empty set
        let result = calculate_iterative(vec![27], version, &taxon_store, &lineage_store, true, 1.0);
        assert_eq!(result, 1);
    }

    #[test]
    fn higher_threshold_produces_ancestor_of_lower_threshold() {
        // A stricter threshold must yield a result at least as ancestral as a looser one.
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let conservative = calculate_iterative(taxa.clone(), version, &taxon_store, &lineage_store, true, 1.0);
        let aggressive   = calculate_iterative(taxa,          version, &taxon_store, &lineage_store, true, 0.5);

        let aggressive_lineage = crate::helpers::lineage_helper::get_lineage_array_numeric(
            aggressive as u32, version, &lineage_store
        );
        assert!(
            conservative == aggressive
                || aggressive_lineage.contains(&conservative)
                || conservative == 1,
            "threshold=1.0 result {conservative} should be an ancestor of threshold=0.5 result {aggressive}"
        );
    }

    #[test]
    fn threshold_zero_descends_past_root() {
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_iterative(taxa, version, &taxon_store, &lineage_store, true, 0.0);
        assert!(result > 1, "threshold=0.0 should descend past the root");
    }
}
