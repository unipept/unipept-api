use datastore::{LineageStore, TaxonStore};

use super::lineage_helper::{
    get_amount_of_ranks, get_genus_index, get_lineage_array_numeric, get_species_index, LineageVersion
};

pub fn calculate_lca(
    taxa: Vec<u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore
) -> i32 {
    let cleaned_taxa = taxa.into_iter().filter(|&taxon_id| taxon_store.is_valid(taxon_id));

    let lineages: Vec<Vec<i32>> = cleaned_taxa
        .into_iter()
        .map(|taxon_id| get_lineage_array_numeric(taxon_id, version, lineage_store))
        .collect();

    let amount_of_ranks = get_amount_of_ranks(version);
    let genus_index = get_genus_index(version);
    let species_index = get_species_index(version);

    for rank in (0..amount_of_ranks).rev() {

        let mut iterator = lineages
            .iter()
            .map(|x| x[rank as usize])
            .filter(|&x| if rank == genus_index || rank == species_index { x > 0 } else { x >= 0 });

        // Check if all elements in the iterator are the same
        let first = iterator.next().unwrap();
        if first > 0 && iterator.all(|item| item == first) {
            return first;
        }
    }

    1 // If no valid lineages

}


#[cfg(test)]
mod tests {
    use datastore::{LineageStore, TaxonStore};
    use crate::helpers::lca_helper::calculate_lca;
    use std::{
        fs::File,
        io::{prelude::*, BufReader},
    };

    use super::super::lineage_helper::LineageVersion;

    fn read_taxa_file() -> Vec<u32> {
        let filename = "../data/taxa_from_400_peptides.txt";
        let file = File::open(filename).expect("no such file");
        let buf = BufReader::new(file);
        buf.lines()
            .map(|l| l.expect("Could not parse line").parse::<u32>().unwrap())
            .collect()
    }


    #[test]
    fn small_test_calculate_lca() {
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        assert_eq!(calculate_lca(
            taxa, version, &taxon_store, &lineage_store), 8287);
    }


    #[test]
    fn test_calculate_lca() {
        let taxa: Vec<u32> = read_taxa_file();
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore = TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore = LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        assert_eq!(calculate_lca(
            taxa, version, &taxon_store, &lineage_store), 1);
    }
}