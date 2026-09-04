pub use errors::IndexError;
use errors::LoadIndexError;
use sa_server::{ActiveSearcher, load_kmer_table_file, load_mapping_file, load_proteins_file, load_suffix_array_file};
pub use sa_index::peptide_search::{ProteinInfo, SearchResult, TaxaSearchResult};
use sa_index::peptide_search::{search_all_peptides, search_all_peptides_taxa};

/// Re-exported so the API can decode the annotations it is handed without taking its own
/// dependency on the index repository, and cannot end up on a different version of it.
pub use fa_compression;

mod errors;

pub struct Index {
    searcher: ActiveSearcher
}

impl Index {
    /// Loads the three index files into the storage backend this binary was compiled for.
    ///
    /// Which backend that is comes from the `mmap` and `preloaded-*` features rather than from an
    /// argument; see `sa_server::backends`.
    /// `kmer_table_file` is optional: when given, the binary search starts from precomputed
    /// bounds instead of the whole array, which is a large saving on short peptides.
    pub fn try_from_files(
        index_file: &str,
        proteins_file: &str,
        mapping_file: &str,
        kmer_table_file: Option<&str>
    ) -> Result<Self, IndexError> {
        eprintln!("Loading proteins from file: {}", proteins_file);
        let proteins =
            load_proteins_file(proteins_file).map_err(|err| LoadIndexError::LoadProteinsErrors(err.to_string()))?;

        eprintln!("Loading suffix array from file: {}", index_file);
        let suffix_array =
            load_suffix_array_file(index_file).map_err(|err| LoadIndexError::LoadSuffixArrayError(err.to_string()))?;

        eprintln!("Loading mapping from file: {}", mapping_file);
        let suffix_to_protein_index =
            load_mapping_file(mapping_file).map_err(|err| LoadIndexError::LoadMappingError(err.to_string()))?;

        // Each file is well-formed on its own; this is the only check that they came from the same
        // sa-builder run. Without it a stale mapping loads, reports ready, and answers wrongly.
        let mut searcher = ActiveSearcher::try_new(suffix_array, proteins, suffix_to_protein_index)
            .map_err(LoadIndexError::MismatchedIndexFiles)?;

        if let Some(kmer_table_file) = kmer_table_file {
            eprintln!("Loading k-mer table from file: {}", kmer_table_file);
            let table =
                load_kmer_table_file(kmer_table_file).map_err(|err| LoadIndexError::LoadKmerTableError(err.to_string()))?;

            // Rejects a table built against a different index, the same way `try_new` rejects a
            // mismatched set of the other three files.
            searcher = searcher.try_with_kmer_table(table).map_err(LoadIndexError::MismatchedIndexFiles)?;
        }

        Ok(Self { searcher })
    }

    /// Searches `peptides` and returns every matching protein.
    ///
    /// The results borrow from both the index and `peptides`: accessions and annotations are read
    /// out of the index rather than copied, and the annotations are still `fa-compression`-encoded.
    pub fn analyse<'a>(
        &'a self,
        peptides: &'a [String],
        equate_il: bool,
        tryptic: bool,
        cutoff: Option<usize>
    ) -> Vec<SearchResult<'a>> {
        search_all_peptides(&self.searcher, peptides, cutoff.unwrap_or(10_000), equate_il, tryptic)
    }

    /// Like `analyse`, but returns only deduplicated taxon IDs per peptide — no accession or
    /// annotation is retrieved at all. The `taxa` list in each result is sorted and deduplicated.
    pub fn analyse_taxa<'a>(
        &self,
        peptides: &'a [String],
        equate_il: bool,
        tryptic: bool,
        cutoff: Option<usize>
    ) -> Vec<TaxaSearchResult<'a>> {
        search_all_peptides_taxa(&self.searcher, peptides, cutoff.unwrap_or(10_000), equate_il, tryptic)
    }

    /// One line naming the storage backend of every index structure in this build.
    pub fn backend_summary() -> String {
        sa_server::backend_summary()
    }
}
