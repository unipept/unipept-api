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
    use datastore::{LineageStore, TaxonStore};
    use fixtures::taxa;
    use tempfile::TempDir;

    use super::super::lineage_helper::LineageVersion;
    use crate::helpers::lca_helper::calculate_lca;

    const VERSION: LineageVersion = LineageVersion::V2;

    /// The shared corpus, written out and loaded.
    ///
    /// The `TempDir` comes back so the caller keeps it alive; dropping it deletes the files.
    fn stores() -> (TempDir, TaxonStore, LineageStore) {
        let dir = TempDir::new().expect("could not create a temporary directory");
        let paths = fixtures::write_datastore_files(dir.path());

        let taxon_store =
            TaxonStore::try_from_file(&paths.taxons.to_string_lossy()).expect("the corpus taxons should load");
        let lineage_store =
            LineageStore::try_from_file(&paths.lineages.to_string_lossy()).expect("the corpus lineages should load");

        (dir, taxon_store, lineage_store)
    }

    /// `pept2lca` takes the index's lightweight taxa path, which reports each distinct taxon once
    /// where the protein path reported one entry per matching protein. That substitution is only
    /// sound if repeats cannot change the answer — `calculate_lca` reduces rank by rank and asks
    /// whether every lineage agrees, so they cannot.
    #[test]
    fn repeated_taxa_do_not_change_the_lca() {
        let (_dir, taxon_store, lineage_store) = stores();

        let distinct = vec![taxa::CROCODYLUS_NILOTICUS, taxa::SPHENODONTIA, taxa::ALOUATTA_SENICULUS];
        let with_repeats = vec![
            taxa::CROCODYLUS_NILOTICUS,
            taxa::SPHENODONTIA,
            taxa::CROCODYLUS_NILOTICUS,
            taxa::ALOUATTA_SENICULUS,
            taxa::SPHENODONTIA,
            taxa::CROCODYLUS_NILOTICUS,
        ];

        assert_eq!(
            calculate_lca(with_repeats, VERSION, &taxon_store, &lineage_store, true),
            calculate_lca(distinct, VERSION, &taxon_store, &lineage_store, true)
        );
    }

    /// Reaching the superclass means stepping over class, which `C. niloticus` does not record.
    #[test]
    fn taxa_that_diverge_at_class_reduce_to_the_superclass() {
        let (_dir, taxon_store, lineage_store) = stores();

        let taxa = vec![taxa::CROCODYLUS_NILOTICUS, taxa::SPHENODONTIA, taxa::ALOUATTA_SENICULUS];

        assert_eq!(calculate_lca(taxa, VERSION, &taxon_store, &lineage_store, true), taxa::SARCOPTERYGII as i32);
    }

    /// Taxa from different domains share no rank, so the reduction runs out and answers root.
    #[test]
    fn taxa_from_different_domains_reduce_to_the_root() {
        let (_dir, taxon_store, lineage_store) = stores();

        let taxa = vec![taxa::CROCODYLUS_NILOTICUS, taxa::AZORHIZOBIUM_CAULINODANS, taxa::BUCHNERA_APHIDICOLA];

        assert_eq!(calculate_lca(taxa, VERSION, &taxon_store, &lineage_store, true), taxa::ROOT as i32);
    }

    /// `validate_taxa=false` lets a caller-supplied taxon ID reach the lineage store, and
    /// `taxa2lca` exposes exactly that. A taxon with no lineage row is not skipped: it reads as a
    /// lineage of zeroes, which agrees with no taxon ID at the ranks that keep zeroes, so it pulls
    /// the answer back to the root. Skipping it instead would answer from the taxa that are left,
    /// which is the reading this function must not take.
    #[test]
    fn a_taxon_without_a_lineage_holds_the_lca_at_the_root() {
        let (_dir, taxon_store, lineage_store) = stores();

        // In neither corpus file, so it stands in for any ID a client can post.
        const UNKNOWN: u32 = 9999999;

        let known = vec![taxa::CROCODYLUS_NILOTICUS, taxa::SPHENODONTIA, taxa::ALOUATTA_SENICULUS];
        let mut with_unknown = known.clone();
        with_unknown.push(UNKNOWN);

        assert_eq!(calculate_lca(known, VERSION, &taxon_store, &lineage_store, false), taxa::SARCOPTERYGII as i32);
        assert_eq!(
            calculate_lca(with_unknown.clone(), VERSION, &taxon_store, &lineage_store, false),
            taxa::ROOT as i32
        );

        // `validate_taxa=true` never reaches that reading: the taxon store rejects the ID first.
        assert_eq!(
            calculate_lca(with_unknown, VERSION, &taxon_store, &lineage_store, true),
            taxa::SARCOPTERYGII as i32
        );
    }

    /// `only_valid_taxa` drops a taxon the taxonomy marks invalid. `Heloderma sp.` is the corpus's
    /// counterexample; with it dropped, only the crocodile is left and the LCA is that taxon.
    #[test]
    fn an_invalid_taxon_is_dropped_only_when_the_caller_asks() {
        let (_dir, taxon_store, lineage_store) = stores();

        let taxa = vec![taxa::CROCODYLUS_NILOTICUS, taxa::HELODERMA];

        assert_eq!(
            calculate_lca(taxa.clone(), VERSION, &taxon_store, &lineage_store, true),
            taxa::CROCODYLUS_NILOTICUS as i32
        );
        assert_ne!(
            calculate_lca(taxa, VERSION, &taxon_store, &lineage_store, false),
            taxa::CROCODYLUS_NILOTICUS as i32,
            "with the filter off the invalid taxon must still count"
        );
    }
}
