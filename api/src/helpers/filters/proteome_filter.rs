use std::collections::HashSet;
use index::ProteinInfo;
use crate::helpers::filters::UniprotFilter;
use datastore::ReferenceProteomeStore;

pub struct ProteomeFilter {
    pub proteins: HashSet<String>
}

impl UniprotFilter for ProteomeFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        self.proteins.contains(protein.uniprot_accession)
    }
}

impl ProteomeFilter {
    pub async fn new(proteomes: HashSet<String>, proteome_store: &ReferenceProteomeStore) -> reqwest::Result<Self> {
        let mut proteins = HashSet::new();

        for proteome in proteomes {
            if let Some(protein_list) = proteome_store.get_proteins(&proteome) {
                proteins.extend(protein_list.iter().map(|s| s.to_string()));
            }
        }

        Ok(ProteomeFilter { proteins })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protein(accession: &str) -> ProteinInfo<'_> {
        ProteinInfo { taxon: 1, uniprot_accession: accession, annotations: &[] }
    }

    #[test]
    fn keeps_only_the_proteins_of_the_selected_proteomes() {
        let filter = ProteomeFilter { proteins: HashSet::from(["P1".to_string()]) };

        assert!(filter.filter(&protein("P1")));
        assert!(!filter.filter(&protein("P2")));
    }
}
