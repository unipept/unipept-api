use std::collections::HashSet;

use index::ProteinInfo;

use crate::helpers::filters::UniprotFilter;

pub struct ProteinFilter {
    pub proteins: HashSet<String>
}

impl UniprotFilter for ProteinFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        self.proteins.contains(protein.uniprot_accession)
    }
}

impl ProteinFilter {
    pub fn new(proteins: HashSet<String>) -> Self {
        ProteinFilter { proteins }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protein(accession: &str) -> ProteinInfo<'_> {
        ProteinInfo { taxon: 1, uniprot_accession: accession, annotations: &[] }
    }

    #[test]
    fn keeps_only_the_listed_accessions() {
        let filter = ProteinFilter::new(HashSet::from(["P1".to_string()]));

        assert!(filter.filter(&protein("P1")));
        assert!(!filter.filter(&protein("P2")));
    }
}
