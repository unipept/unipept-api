use datastore::{LineageStore, TaxonStore};

use super::lineage_helper::{
    get_amount_of_ranks, get_genus_index, get_lineage_array, get_species_index, LineageVersion
};

pub fn calculate_lca(
    taxa: Vec<u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore
) -> i32 {
    let cleaned_taxa: Vec<u32> = taxa.into_iter().filter(|&taxon_id| taxon_store.is_valid(taxon_id)).collect();

    let lineages = prepare_lineages(cleaned_taxa, version, lineage_store);

    let amount_of_ranks = get_amount_of_ranks(version);
    let genus_index = get_genus_index(version);
    let species_index = get_species_index(version);

    for rank in (0..amount_of_ranks).rev() {
        let final_rank: u8 = rank;

        let mut iterator = lineages
            .iter()
            .map(|x| x[final_rank as usize])
            .filter(|&x| if final_rank == genus_index || final_rank == species_index { x > 0 } else { x >= 0 });

        // Check if all elements in the iterator are the same
        let first = iterator.next().unwrap();
        if first != 0 && iterator.all(|item| item == first) {
            return first;
        }
    }

    -1 // If no valid lineages

}

fn prepare_lineages(taxa: Vec<u32>, version: LineageVersion, lineage_store: &LineageStore) -> Vec<Vec<i32>> {
    taxa.into_iter()
        .map(|taxon_id| {
            get_lineage_array(taxon_id, version, lineage_store).into_iter().map(|x| x.unwrap_or(0)).collect()
        })
        .collect()
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