use std::collections::HashSet;
use datastore::LineageStore;
use index::ProteinInfo;
use crate::helpers::filters::UniprotFilter;
use crate::helpers::lineage_helper::{get_lineage_array, LineageVersion};

pub struct TaxaFilter<'a> {
    pub taxa: HashSet<u32>,
    lineage_store: &'a LineageStore
}

impl UniprotFilter for TaxaFilter<'_> {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        get_lineage_array(protein.taxon, LineageVersion::V2, self.lineage_store)
            .iter()
            .flatten()
            .any(|ancestor| self.taxa.contains(&(ancestor.abs() as u32)))
    }
}

impl<'a> TaxaFilter<'a> {
    pub fn new(taxa: HashSet<u32>, lineage_store: &'a LineageStore) -> Self {
        TaxaFilter { taxa, lineage_store }
    }
}
