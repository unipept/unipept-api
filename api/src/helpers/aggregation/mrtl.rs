use std::collections::HashSet;
use datastore::{LineageStore, TaxonStore};

use crate::helpers::lineage_helper::{get_lineage_array_numeric, LineageVersion};

pub fn calculate_mrtl(
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

    cleaned
        .iter()
        .copied()
        .map(|id| {
            let score = get_lineage_array_numeric(id, version, lineage_store)
                .iter()
                .filter(|&&anc| anc > 0 && anc as u32 != id && taxon_set.contains(&(anc as u32)))
                .count();
            (id, score)
        })
        .max_by_key(|&(_, score)| score)
        .map(|(id, _)| id as i32)
        .unwrap_or(1)
}


#[cfg(test)]
mod tests {
    use datastore::{LineageStore, TaxonStore};
    use crate::helpers::aggregation::mrtl::calculate_mrtl;
    use std::{
        fs::File,
        io::{prelude::*, BufReader},
    };

    use crate::helpers::lineage_helper::LineageVersion;

    fn load_stores() -> (TaxonStore, LineageStore) {
        let taxon_store = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");
        (taxon_store, lineage_store)
    }

    fn read_taxa_file() -> Vec<u32> {
        let filename = "../data/taxa_from_400_peptides.txt";
        let file = File::open(filename).expect("no such file");
        let buf = BufReader::new(file);
        buf.lines()
            .map(|l| l.expect("Could not parse line").parse::<u32>().unwrap())
            .collect()
    }

    #[test]
    fn small_test_calculate_mrtl() {
        // 8501 and 8505 are both children of 8287, 9503 is unrelated
        // MRTL should return whichever of {8501, 8505, 9503} has most ancestors in the set
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_mrtl(taxa.clone(), version, &taxon_store, &lineage_store, true);

        // Result must always be a member of the input list (unlike LCA which returned 8287)
        assert!(taxa.contains(&(result as u32)), "MRTL result {result} not in input list");
    }

    #[test]
    fn single_taxon_returns_itself() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_mrtl(vec![8501], version, &taxon_store, &lineage_store, true);
        assert_eq!(result, 8501);
    }

    #[test]
    fn empty_after_filter_returns_root() {
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        // taxon 27 is invalid, so with only_valid_taxa=true we get an empty set
        let result = calculate_mrtl(vec![27], version, &taxon_store, &lineage_store, true);
        assert_eq!(result, 1);
    }

    #[test]
    fn result_always_in_input_list() {
        let taxa = read_taxa_file();
        let version = LineageVersion::V2;
        let (taxon_store, lineage_store) = load_stores();

        let result = calculate_mrtl(taxa.clone(), version, &taxon_store, &lineage_store, true);

        // The defining property of MRTL: result is always in the input
        let valid_taxa: Vec<u32> = taxa.into_iter().filter(|&id| taxon_store.is_valid(id)).collect();
        assert!(
            valid_taxa.contains(&(result as u32)) || result == 1,
            "MRTL result {result} not in filtered input list"
        );
    }
}
