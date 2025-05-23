use std::{
    fs::File,
    io::{BufReader, Read}
};

pub use errors::IndexError;
use errors::LoadIndexError;
use sa_compression::load_compressed_suffix_array;
pub use sa_index::peptide_search::ProteinInfo;
pub use sa_index::peptide_search::SearchResult;
use sa_index::{
    binary::load_suffix_array,
    peptide_search::{search_all_peptides},
    sa_searcher::BitVecSearcher,
    SuffixArray
};
use sa_mappings::proteins::Proteins;

mod errors;

pub struct Index {
    searcher: BitVecSearcher
}

impl Index {
    pub fn try_from_files(index_file: &str, proteins_file: &str) -> Result<Self, IndexError> {

        let proteins = Proteins::try_from_database_file(proteins_file)
            .map_err(|_| LoadIndexError::LoadProteinsErrors(
                LoadIndexError::FileNotFound(proteins_file.to_string()).to_string(),
            ))?;

        let suffix_array =
            load_index_file(index_file).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?;

        let searcher = BitVecSearcher::new(suffix_array, proteins);

        Ok(Self { searcher })
    }

    pub fn analyse(&self, peptides: &Vec<String>, equate_il: bool, tryptic: bool, cutoff: Option<usize>) -> Vec<SearchResult> {
        search_all_peptides(&self.searcher, peptides, cutoff.unwrap_or(10_000), equate_il, tryptic)
    }
}

fn load_index_file(index_file: &str) -> Result<SuffixArray, LoadIndexError> {
    // Open the suffix array file
    let mut sa_file = File::open(index_file).map_err(
        |_| LoadIndexError::FileNotFound(index_file.to_string())
    )?;

    // Create a buffer reader for the file
    let mut reader = BufReader::new(&mut sa_file);

    // Read the bits per value from the binary file (1 byte)
    let mut bits_per_value_buffer = [0_u8; 1];
    reader.read_exact(&mut bits_per_value_buffer).map_err(|_| {
        LoadIndexError::LoadSuffixArrayError("Could not read the flags from the binary file".to_string())
    })?;
    let bits_per_value = bits_per_value_buffer[0];

    if bits_per_value == 64 {
        Ok(load_suffix_array(&mut reader).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?)
    } else {
        Ok(load_compressed_suffix_array(&mut reader, bits_per_value as usize)
            .map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?)
    }
}
