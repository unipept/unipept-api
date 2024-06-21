use std::{fs::File, io::{BufReader, Read}};

use sa_compression::load_compressed_suffix_array;
use sa_index::{binary::load_suffix_array, peptide_search::{analyse_all_peptides, OutputData, SearchResultWithAnalysis}, sa_searcher::Searcher, suffix_to_protein_index::SparseSuffixToProtein, SuffixArray};
use sa_mappings::{functionality::FunctionAggregator, proteins::Proteins, taxonomy::{AggregationMethod, TaxonAggregator}};

pub struct Index {
    searcher: Searcher
}

impl Index {
    pub fn try_from_files(
        index_file: &str,
        proteins_file: &str,
        taxonomy_file: &str
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let suffix_array = load_index_file(index_file)?;

        let taxon_id_calculator =
            TaxonAggregator::try_from_taxonomy_file(taxonomy_file, AggregationMethod::LcaStar)?;

        let function_aggregator = FunctionAggregator {};

        let proteins = Proteins::try_from_database_file(proteins_file, &taxon_id_calculator)?;
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

fn load_index_file(index_file: &str) -> Result<SuffixArray, Box<dyn std::error::Error>> {
    // Open the suffix array file
    let mut sa_file = File::open(index_file)?;

    // Create a buffer reader for the file
    let mut reader = BufReader::new(&mut sa_file);

    // Read the bits per value from the binary file (1 byte)
    let mut bits_per_value_buffer = [0_u8; 1];
    reader
        .read_exact(&mut bits_per_value_buffer)
        .map_err(|_| "Could not read the flags from the binary file")?;
    let bits_per_value = bits_per_value_buffer[0];

    if bits_per_value == 64 {
        load_suffix_array(&mut reader)
    } else {
        load_compressed_suffix_array(&mut reader, bits_per_value as usize)
    }
}
