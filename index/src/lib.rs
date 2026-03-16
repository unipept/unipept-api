pub use errors::IndexError;
use errors::LoadIndexError;
use sa_server::{load_mapping_file, load_proteins_file, load_suffix_array_file};
pub use sa_index::peptide_search::ProteinInfo;
pub use sa_index::peptide_search::SearchResult;
use sa_index::{
    peptide_search::{search_all_peptides},
};
use sa_index::sa_searcher::Searcher;
use sa_index::suffix_to_protein_index::SuffixToProteinIndex;

mod errors;

pub struct Index {
    searcher: Searcher
}

impl Index {
    pub fn try_from_files(index_file: &str, proteins_file: &str, mapping_file: &str, use_mmap: bool) -> Result<Self, IndexError> {
        eprintln!("Loading proteins from file: {}", proteins_file);
        let proteins =
            load_proteins_file(proteins_file, use_mmap).map_err(|err| LoadIndexError::LoadProteinsErrors(err.to_string()))?;

        eprintln!("Loading suffix array from file: {}", index_file);
        let suffix_array =
            load_suffix_array_file(index_file, use_mmap).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?;

        eprintln!("Creating searcher");
        let suffix_to_protein_mapping =
            load_mapping_file(mapping_file, use_mmap).map_err(|err| LoadIndexError::LoadProteinsErrors(err.to_string()))?;

        let searcher = Searcher::new(suffix_array, proteins, suffix_to_protein_mapping.0);

        Ok(Self { searcher })
    }

    pub fn analyse(&self, peptides: &Vec<String>, equate_il: bool, tryptic: bool, cutoff: Option<usize>) -> Vec<SearchResult> {
        search_all_peptides(&self.searcher, peptides, cutoff.unwrap_or(10_000), equate_il, tryptic)
    }
}
