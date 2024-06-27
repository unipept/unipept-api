use std::{fs::File, io::{BufReader, Read}};

pub use errors::IndexError;
use errors::LoadIndexError;
use sa_compression::load_compressed_suffix_array;
use sa_index::{binary::load_suffix_array, peptide_search::{analyse_all_peptides, OutputData, SearchResultWithAnalysis}, sa_searcher::Searcher, suffix_to_protein_index::SparseSuffixToProtein, SuffixArray};
use sa_mappings::{functionality::FunctionAggregator, proteins::Proteins, taxonomy::{AggregationMethod, TaxonAggregator}};

mod errors;

pub struct Index {
    searcher: Searcher
}

impl Index {
    pub fn try_from_files(
        index_file: &str,
        proteins_file: &str,
        taxonomy_file: &str
    ) -> Result<Self, IndexError> {
        let suffix_array = load_index_file(index_file)
            .map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?;

        let taxon_id_calculator =
            TaxonAggregator::try_from_taxonomy_file(taxonomy_file, AggregationMethod::LcaStar)
                .map_err(|err| LoadIndexError::LoadTaxonomyError(err.to_string()))?;

        let function_aggregator = FunctionAggregator {};

        let proteins = Proteins::try_from_database_file(proteins_file, &taxon_id_calculator)
            .map_err(|err| LoadIndexError::LoadProteinsErrors(err.to_string()))?;
        let suffix_index_to_protein = Box::new(SparseSuffixToProtein::new(&proteins.input_string));

        let searcher = Searcher::new(
            suffix_array,
            suffix_index_to_protein,
            proteins,
            taxon_id_calculator,
            function_aggregator
        );

        Ok(Self { searcher })
    }

    pub fn analyse(&self, peptides: &Vec<String>, equate_il: bool) -> OutputData<SearchResultWithAnalysis> {
        analyse_all_peptides(
            &self.searcher, 
            peptides, 
            10_000, 
            equate_il, 
            false
        )
    }
}

fn load_index_file(index_file: &str) -> Result<SuffixArray, LoadIndexError> {
    // Open the suffix array file
    let mut sa_file = File::open(index_file)?;

    // Create a buffer reader for the file
    let mut reader = BufReader::new(&mut sa_file);

    // Read the bits per value from the binary file (1 byte)
    let mut bits_per_value_buffer = [0_u8; 1];
    reader
        .read_exact(&mut bits_per_value_buffer)
        .map_err(|_| LoadIndexError::LoadSuffixArrayError("Could not read the flags from the binary file".to_string()))?;
    let bits_per_value = bits_per_value_buffer[0];

    if bits_per_value == 64 {
        Ok(load_suffix_array(&mut reader).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?)
    } else {
        Ok(load_compressed_suffix_array(&mut reader, bits_per_value as usize).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?)
    }
}
