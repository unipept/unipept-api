use datastore::{Lineage, LineageStore, TaxonStore};

use super::lineage_helper::{LineageVersion, get_amount_of_ranks, get_genus_index, get_species_index};

/// Takes the taxa as an iterator rather than a `Vec`: the callers that hold one still pass it, and
/// the ones that build the list only to hand it over no longer allocate it.
pub fn calculate_lca(
    taxa: impl IntoIterator<Item = u32>,
    version: LineageVersion,
    taxon_store: &TaxonStore,
    lineage_store: &LineageStore,
    only_valid_taxa: bool
) -> i32 {
    // A taxon the lineage store does not know still counts, and reads as a lineage of zeroes. Zero
    // agrees with no taxon ID, so such a taxon holds the result at the root, which is what the
    // per-taxon array this loop used to build did.
    let unknown = Lineage::default();

    // Borrowed, not copied: the loop below reads each lineage once per rank, and the store already
    // holds them. Building an owned array per taxon allocated once per input taxon, of which a
    // single `pept2data` request can carry hundreds of thousands.
    let lineages: Vec<&Lineage> = taxa
        .into_iter()
        .filter(|&taxon_id| !only_valid_taxa || taxon_store.is_valid(taxon_id))
        .map(|taxon_id| lineage_store.get(taxon_id).map(|lineage| lineage.as_ref()).unwrap_or(&unknown))
        .collect();

    let amount_of_ranks = get_amount_of_ranks(version);
    let genus_index = get_genus_index(version);
    let species_index = get_species_index(version);

    for rank in (0..amount_of_ranks).rev() {
        let mut iterator = lineages
            .iter()
            // The same reading `get_lineage_array_numeric` gives a rank: -1 and an absent rank both
            // read as 0, and a negative ID is reported as its absolute value.
            .map(|lineage| lineage.get_rank(rank as usize).filter(|&id| id != -1).map(i32::abs).unwrap_or(0))
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

    /// `validate_taxa=false` lets a caller-supplied taxon ID reach the lineage store, and
    /// `taxa2lca` exposes exactly that. A taxon with no lineage row is not skipped: it reads as a
    /// lineage of zeroes, which agrees with no taxon ID at the ranks that keep zeroes, so it pulls
    /// the answer back to the root. Skipping it instead would answer from the taxa that are left,
    /// which is the reading this function must not take.
    #[test]
    fn a_taxon_without_a_lineage_holds_the_lca_at_the_root() {
        let version: LineageVersion = LineageVersion::V2;
        let taxon_store: TaxonStore =
            TaxonStore::try_from_file("../data/taxons_subset_10000.tsv").expect("Reading the file failed");
        let lineage_store: LineageStore =
            LineageStore::try_from_file("../data/lineages_subset_10000.tsv").expect("Reading the file failed");

        // Not a row in either subset, so it stands in for any ID a client can post.
        let unknown: u32 = 9999999;

        assert_eq!(calculate_lca(vec![8501, 8505, 9503], version, &taxon_store, &lineage_store, false), 8287);
        assert_eq!(calculate_lca(vec![8501, 8505, 9503, unknown], version, &taxon_store, &lineage_store, false), 1);

        // `validate_taxa=true` never reaches that reading: the taxon store rejects the ID first.
        assert_eq!(calculate_lca(vec![8501, 8505, 9503, unknown], version, &taxon_store, &lineage_store, true), 8287);
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
