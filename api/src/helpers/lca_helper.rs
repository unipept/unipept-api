use datastore::{LineageStore, TaxonStore};

use super::lineage_helper::{
    LineageVersion, get_amount_of_ranks, get_genus_index, get_lineage_array_numeric, get_species_index
};

pub fn calculate_lca(
    taxa: Vec<u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore,
    only_valid_taxa: bool
) -> i32 {
    let cleaned_taxa = taxa.into_iter().filter(|&taxon_id| !only_valid_taxa || taxon_store.is_valid(taxon_id));

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
        if let Some(first) = iterator.next()
            && first > 0
            && iterator.all(|item| item == first)
        {
            return first;
        }
    }

    1 // If no valid lineages
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufReader, prelude::*}
    };

    use datastore::{LineageStore, TaxonStore};

    use super::super::lineage_helper::LineageVersion;
    use crate::helpers::lca_helper::calculate_lca;

    fn read_taxa_file() -> Vec<u32> {
        let filename = "../data/taxa_from_400_peptides.txt";
        let file = File::open(filename).expect("no such file");
        let buf = BufReader::new(file);
        buf.lines().map(|l| l.expect("Could not parse line").parse::<u32>().unwrap()).collect()
    }

    /// `pept2lca` takes the index's lightweight taxa path, which reports each distinct taxon once
    /// where the protein path reported one entry per matching protein. That substitution is only
    /// sound if repeats cannot change the answer — `calculate_lca` reduces rank by rank and asks
    /// whether every lineage agrees, so they cannot.
    #[test]
    fn repeated_taxa_do_not_change_the_lca() {
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore =
            TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore =
            LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        let distinct: Vec<u32> = vec![8501, 8505, 9503];
        let with_repeats: Vec<u32> = vec![8501, 8505, 8501, 9503, 8505, 8501];

        assert_eq!(
            calculate_lca(with_repeats, version, &taxon_store, &lineage_store, true),
            calculate_lca(distinct, version, &taxon_store, &lineage_store, true)
        );
    }

    #[test]
    fn small_test_calculate_lca() {
        let taxa: Vec<u32> = vec![8501, 8505, 9503];
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore =
            TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore =
            LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        assert_eq!(calculate_lca(taxa, version, &taxon_store, &lineage_store, true), 8287);
    }

    #[test]
    fn test_calculate_lca() {
        let taxa: Vec<u32> = read_taxa_file();
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore =
            TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore =
            LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        assert_eq!(calculate_lca(taxa, version, &taxon_store, &lineage_store, true), 1);
    }

    #[test]
    fn test_calculate_lca_validate() {
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore =
            TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore =
            LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        assert_eq!(calculate_lca(vec![27], version, &taxon_store, &lineage_store, true), 1);
        assert_eq!(calculate_lca(vec![27], version, &taxon_store, &lineage_store, false), 27);
    }
}
