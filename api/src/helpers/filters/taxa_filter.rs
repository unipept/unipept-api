use std::collections::HashSet;

use datastore::LineageStore;
use index::ProteinInfo;

use crate::helpers::{filters::UniprotFilter, lineage_helper::reported_ranks};

pub struct TaxaFilter<'a> {
    pub taxa: HashSet<u32>,
    lineage_store: &'a LineageStore
}

impl UniprotFilter for TaxaFilter<'_> {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        reported_ranks(protein.taxon, self.lineage_store)
            .flatten()
            .any(|ancestor| self.taxa.contains(&(ancestor.unsigned_abs())))
    }
}

impl<'a> TaxaFilter<'a> {
    pub fn new(taxa: HashSet<u32>, lineage_store: &'a LineageStore) -> Self {
        TaxaFilter { taxa, lineage_store }
    }
}
