use std::collections::HashSet;
use index::ProteinInfo;
use crate::helpers::filters::UniprotFilter;
use datastore::ReferenceProteomeStore;

pub struct ProteomeFilter {
    pub proteins: HashSet<String>
}

impl UniprotFilter for ProteomeFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        self.proteins.contains(&protein.uniprot_accession)
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
