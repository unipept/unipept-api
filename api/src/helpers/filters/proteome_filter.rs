use std::collections::HashSet;
use index::ProteinInfo;
use crate::helpers::filters::UniprotFilter;

pub struct ProteomeFilter {
    pub proteins: HashSet<String>
}

impl UniprotFilter for ProteomeFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        self.proteins.contains(&protein.uniprot_accession)
    }
}

impl ProteomeFilter {
    pub async fn new(proteomes: HashSet<String>) -> reqwest::Result<Self> {
        let mut proteins = HashSet::new();

        for proteome in proteomes {
            proteins.extend(fetch_proteome(proteome).await?);
        }

        Ok(ProteomeFilter { proteins })
    }
}

async fn fetch_proteome(proteome: String) -> reqwest::Result<HashSet<String>> {
    let url = format!("https://rest.uniprot.org/uniprotkb/stream?fields=accession&format=list&query=(proteome:{})", proteome);
    let proteins_string = reqwest::get(url).await?.text().await?;
    Ok(proteins_string.lines().map(|line| line.to_string()).collect())
}
