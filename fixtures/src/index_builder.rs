//! Building index files from the protein corpus, the way `sa-builder` does.
//!
//! Nothing binary is committed. The `.bin` files are artifacts of `unipept-index`, which this
//! workspace tracks as a git dependency with no `rev` in the manifest, so a committed one would be
//! an artifact of another repository at a version nothing here records — and a stale index does not
//! always fail to load, it can load and answer wrongly.
//!
//! Building instead costs one `libsais` run over a few hundred residues and cannot go stale.

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf}
};

use binary_traits::WriteBinary;
use protein_metadata::{InMemoryProteins, ProteinsBackend as _};
use protein_text::ProteinTextBackend as _;
use sa_builder::{SAConstructionAlgorithm, build_ssa};
use sa_index::{KmerTable, array::dump_suffix_array, suffix_to_protein_index::DenseSuffixToProtein};

/// The files `Index::try_from_files` takes, in its argument order.
pub struct IndexPaths {
    pub suffix_array: PathBuf,
    pub proteins: PathBuf,
    pub mapping: PathBuf,
    /// Written only by [`build_index_files_with_kmer_table`].
    ///
    /// [`build_index_files`] leaves it absent, because `Index::try_from_files` treats a missing
    /// table as "search the whole suffix array" and the table is sized by the alphabet raised to
    /// `k` rather than by the corpus. At a small `k` it is cheap enough to build for the tests that
    /// need one.
    pub kmer_table: PathBuf
}

/// Builds a suffix array, mapping and protein table from [`crate::PROTEINS_TSV`] into `dir`.
///
/// The suffix array is uncompressed at sparseness 1: a corpus this small gains nothing from either
/// and both would only stand between a failing assertion and its cause.
pub fn build_index_files(dir: &Path) -> IndexPaths {
    build_index_files_from(dir, crate::PROTEINS_TSV)
}

/// As [`build_index_files`], but also writes a k-mer table built at `k`.
///
/// The table costs `AMINO_ACID_COUNT^k * 16` bytes and nothing in the corpus, so keep `k` small —
/// 3 is a few hundred kilobytes, the default of 5 is over a hundred megabytes.
pub fn build_index_files_with_kmer_table(dir: &Path, k: usize) -> IndexPaths {
    build(dir, crate::PROTEINS_TSV, Some(k))
}

/// Builds an index from arbitrary protein rows, for tests that need a *second*, deliberately
/// different index — checking that files from two builds are refused when mixed, for instance.
pub fn build_index_files_from(dir: &Path, proteins_tsv: &str) -> IndexPaths {
    build(dir, proteins_tsv, None)
}

/// Both of the above at once: arbitrary proteins *and* a k-mer table, for the case that needs a
/// table belonging to a demonstrably different index.
pub fn build_index_files_from_with_kmer_table(dir: &Path, proteins_tsv: &str, k: usize) -> IndexPaths {
    build(dir, proteins_tsv, Some(k))
}

fn build(dir: &Path, proteins_tsv: &str, kmer_size: Option<usize>) -> IndexPaths {
    std::fs::create_dir_all(dir).expect("could not create the fixture directory");
    let tsv = dir.join("proteins.tsv");
    std::fs::write(&tsv, proteins_tsv).expect("could not write the protein corpus");

    let proteins = InMemoryProteins::load_from_tsv(tsv.to_str().expect("the fixture path is valid UTF-8"))
        .expect("the protein corpus should load");

    let text: Vec<u8> = proteins.text().iter().collect();
    let text_len = proteins.text().len();

    let suffix_array = build_ssa(text, &SAConstructionAlgorithm::LibSais, 1).expect("the suffix array should build");

    let paths = IndexPaths {
        suffix_array: dir.join("sa.bin"),
        proteins: dir.join("proteins.bin"),
        mapping: dir.join("mapping.bin"),
        kmer_table: dir.join("kmer_table.bin")
    };

    // Built before the array is consumed by `dump_suffix_array` below.
    let kmer_table =
        kmer_size.map(|k| KmerTable::build_from_raw_sa(&suffix_array, text_len, |i| proteins.text().get(i), k));

    write_to(&paths.suffix_array, |writer| {
        dump_suffix_array(suffix_array, 1, writer).expect("the suffix array should serialise");
    });

    if let Some(table) = kmer_table {
        write_to(&paths.kmer_table, |writer| {
            table.write_binary(writer).expect("the k-mer table should serialise");
        });
    }

    write_to(&paths.mapping, |writer| {
        DenseSuffixToProtein::from_text_parts(text_len, |i| proteins.text().get(i))
            .write_binary(writer)
            .expect("the mapping should serialise");
    });

    // Last, because `write_binary` consumes the proteins and the text is borrowed from them above.
    write_to(&paths.proteins, |writer| {
        proteins.write_binary(writer).expect("the protein table should serialise");
    });

    paths
}

fn write_to(path: &Path, write: impl FnOnce(&mut BufWriter<File>)) {
    let file = File::create(path).unwrap_or_else(|err| panic!("could not create {}: {}", path.display(), err));
    let mut writer = BufWriter::new(file);
    write(&mut writer);
    writer.flush().unwrap_or_else(|err| panic!("could not flush {}: {}", path.display(), err));
}
