use std::collections::HashSet;
use index::ProteinInfo;
use crate::helpers::lca_helper::filters::UniprotFilter;

pub struct ProteinFilter {
    pub proteins: HashSet<String>
}

impl UniprotFilter for ProteinFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        self.proteins.contains(&protein.uniprot_accession)
    }
}

impl ProteinFilter {
    pub fn new(proteins: HashSet<String>) -> Self {
        ProteinFilter { proteins }
    }
}
