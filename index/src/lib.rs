pub use errors::IndexError;
use errors::LoadIndexError;
use sa_server::load_suffix_array_file;
pub use sa_index::peptide_search::ProteinInfo;
pub use sa_index::peptide_search::SearchResult;
use sa_index::{
    peptide_search::{search_all_peptides},
    sa_searcher::BitVecSearcher,
};
use sa_mappings::proteins::Proteins;

mod errors;

pub struct Index {
    searcher: BitVecSearcher
}

impl Index {
    pub fn try_from_files(index_file: &str, proteins_file: &str) -> Result<Self, IndexError> {
        eprintln!("Loading proteins from file: {}", proteins_file);
        let proteins = Proteins::try_from_database_file(proteins_file)
            .map_err(|_| LoadIndexError::LoadProteinsErrors(
                LoadIndexError::FileNotFound(proteins_file.to_string()).to_string(),
            ))?;

        eprintln!("Loading suffix array from file: {}", index_file);
        let suffix_array =
            load_suffix_array_file(index_file, true).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?;

        eprintln!("Creating searcher");
        let searcher = BitVecSearcher::new(suffix_array, proteins);

        Ok(Self { searcher })
    }

    pub fn analyse(&self, peptides: &Vec<String>, equate_il: bool, tryptic: bool, cutoff: Option<usize>) -> Vec<SearchResult> {
        search_all_peptides(&self.searcher, peptides, cutoff.unwrap_or(10_000), equate_il, tryptic)
    }
}
