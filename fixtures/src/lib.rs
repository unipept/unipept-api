//! The shared test corpus.
//!
//! Every crate in this workspace that needs realistic input reads it from here, rather than
//! keeping a corpus of its own. The reason is composition: `datastore`, `index` and `database`
//! are eventually assembled into one `AppState`, and three independently invented corpora would
//! disagree about which taxa exist. Endpoint tests over such a state return empty results for
//! every peptide, pass, and prove nothing — a failure that is invisible in review, because an
//! empty result for an unfamiliar peptide looks entirely plausible.
//!
//! Two layers make up the corpus:
//!
//! - **Taxonomy** — `data/taxons.tsv` and `data/lineages.tsv`: twenty-one real NCBI rows, copied
//!   verbatim out of the 10,000-taxon pair under the repository's own `data/`. The set is closed
//!   under ancestry — every taxon a protein names, every ancestor those taxa's lineages record, and
//!   root — so it is small enough to read in full, which is what makes an expected LCA checkable by
//!   eye rather than by rerunning the code.
//! - **Proteins** — `data/proteins.tsv`: twelve rows referencing only taxa from that subset.
//!
//! To regenerate the taxonomy after changing the proteins: take the taxon column of
//! `proteins.tsv`, union it with every non-`\N` rank id on those taxa's rows in
//! `data/lineages_subset_10000.tsv`, add taxon 1, and copy the matching rows out of both
//! `data/*_subset_10000.tsv` files. Copy them rather than rewriting them — the fifth taxon column
//! is a raw `0x01`/`0x00` byte, not text, and fifteen of the ancestors have a lineage row but no
//! taxon row, which is a property of the 10,000-row sample and not an error.
//!
//! Writers panic rather than returning errors. A fixture that cannot be written to a temporary
//! directory is a broken harness, not a condition a test should handle; this is the opposite of
//! the policy for *parsing* corpus files, where malformed input must always surface as an error.

use std::{
    fs,
    path::{Path, PathBuf}
};

/// The protein corpus: `accession`, `taxon`, `sequence`, `annotations`, tab separated.
///
/// The layout is the one `sa-builder` reads; see `protein-metadata` in the index repository.
pub const PROTEINS_TSV: &str = include_str!("../data/proteins.tsv");

/// EC numbers: `id`, `ec`, `name`. Keys are bare, without the `EC:` prefix the annotations carry.
pub const EC_NUMBERS_TSV: &str = include_str!("../data/ec_numbers.tsv");

/// GO terms: `id`, `go`, `namespace`, `name`. Keys keep their `GO:` prefix, unlike EC and InterPro.
pub const GO_TERMS_TSV: &str = include_str!("../data/go_terms.tsv");

/// InterPro entries: `id`, `ipr`, `type`, `name`. Keys are bare, without the `IPR:` prefix.
pub const INTERPRO_ENTRIES_TSV: &str = include_str!("../data/interpro_entries.tsv");

/// Reference proteomes: `id`, `accession`, `taxon`, `count`, `proteins`.
///
/// The final column is semicolon separated, which is how `ReferenceProteomeStore` splits it.
pub const PROTEOMES_TSV: &str = include_str!("../data/proteomes.tsv");

/// Sample datasets, in the shape `SampleStore` deserialises.
pub const SAMPLEDATA_JSON: &str = include_str!("../data/sampledata.json");

/// Index version string, written to `.version`.
pub const VERSION: &str = include_str!("../data/version.txt");

/// Taxa: `id`, `name`, `rank`, `parent`, validity. The rank strings are the ones
/// `LineageRank::from_str` accepts, and the fifth column is a raw `0x01`/`0x00` byte.
pub const TAXONS_TSV: &str = include_str!("../data/taxons.tsv");

/// Lineages: a taxon id followed by 28 rank columns, `\N` where the taxonomy records nothing.
pub const LINEAGES_TSV: &str = include_str!("../data/lineages.tsv");

/// Every accession in the corpus, in file order.
///
/// Database mocks key their canned responses on these, so that an index built from the corpus and
/// a mocked OpenSearch agree about which proteins exist.
pub const ACCESSIONS: [&str; 12] = [
    "P00001", "P00002", "P00003", "P00004", "P00005", "P00006", "P00007", "P00008", "P00009", "P00010", "P00011",
    "P00012"
];

/// Taxa the corpus references, all present in `data/taxons.tsv`.
///
/// The relationships between them are what the LCA scenarios rest on, so they are named rather
/// than written as bare numbers at the assertion site.
pub mod taxa {
    /// The root of the taxonomy; the answer when no rank is shared.
    pub const ROOT: u32 = 1;
    /// `Azorhizobium caulinodans`, a bacterium — the other side of every cross-domain scenario.
    pub const AZORHIZOBIUM_CAULINODANS: u32 = 7;
    /// `Buchnera aphidicola`, a second bacterium.
    pub const BUCHNERA_APHIDICOLA: u32 = 9;
    /// Superclass `Sarcopterygii`, shared by the crocodiles and the monkey below. The LCA once
    /// their classes diverge, and the deepest rank both lineages actually record.
    pub const SARCOPTERYGII: u32 = 8287;
    /// `Crocodylidae`, the family above the genus.
    pub const CROCODYLIDAE: u32 = 8493;
    /// `Crocodylus`, the genus shared by the three species below.
    pub const CROCODYLUS: u32 = 8500;
    /// `Crocodylus niloticus`. Its lineage records no class, which is the gap scenario.
    pub const CROCODYLUS_NILOTICUS: u32 = 8501;
    /// `Crocodylus porosus`.
    pub const CROCODYLUS_POROSUS: u32 = 8502;
    /// `Crocodylus novaeguineae`.
    pub const CROCODYLUS_NOVAEGUINEAE: u32 = 8503;
    /// `Sphenodontia`, an order — a taxon that is not species rank.
    pub const SPHENODONTIA: u32 = 8505;
    /// `Alouatta seniculus`, a mammal; diverges from the crocodiles at class.
    pub const ALOUATTA_SENICULUS: u32 = 9503;
}

/// Marker peptides, each placed to make one branch fire.
///
/// A branch no fixture peptide reaches has no test, however many proteins the corpus holds, so
/// these are the corpus's actual specification — the sequences around them are filler.
pub mod peptides {
    /// Occurs in one protein only, of `Crocodylus niloticus`. LCA is that taxon itself.
    pub const UNIQUE: &str = "AWDIQNGK";
    /// Shared by `C. niloticus` and `C. porosus`, so the LCA reduces to the genus `Crocodylus`.
    pub const GENUS_SHARED: &str = "TVGENYSAK";
    /// Shared by `C. niloticus` and `Alouatta seniculus`, which diverge at class: the LCA is the
    /// superclass, and reaching it requires stepping over a rank one lineage does not record.
    pub const SUPERCLASS_SHARED: &str = "LHCDMPTFAK";
    /// Shared across domains, by `C. niloticus` and a bacterium. Reduces all the way to root.
    pub const ROOT_SHARED: &str = "GISVNDAWEK";
    /// Differs from [`IL_LEUCINE`] only at the first residue.
    ///
    /// With `equate_il` off this finds `C. porosus` alone; with it on, `C. novaeguineae` too, and
    /// the LCA moves from the species to the genus. One flag, one observable difference.
    pub const IL_ISOLEUCINE: &str = "IPTLNAGVK";
    /// The leucine counterpart of [`IL_ISOLEUCINE`], in `C. novaeguineae`.
    pub const IL_LEUCINE: &str = "LPTLNAGVK";
    /// Present in seven of the twelve proteins, spanning both domains — enough occurrences for a
    /// low cutoff to bite and set `cutoff_used`.
    pub const COMMON: &str = "AAGGK";
    /// In no protein. Distinguishes an empty result from an error.
    pub const ABSENT: &str = "WWWWWWWWWW";
}

/// Paths to a written-out set of datastore files, in the order `DataStore::try_from_files` takes.
pub struct DataStorePaths {
    pub version: PathBuf,
    pub sampledata: PathBuf,
    pub ec_numbers: PathBuf,
    pub go_terms: PathBuf,
    pub interpro_entries: PathBuf,
    pub proteomes: PathBuf,
    pub lineages: PathBuf,
    pub taxons: PathBuf
}

/// Writes all eight datastore files into `dir` and returns their paths.
///
/// Every returned path is inside `dir`, including the taxonomy, so a caller can delete the
/// directory and be certain nothing of the fixture survives.
pub fn write_datastore_files(dir: &Path) -> DataStorePaths {
    create_dir(dir);

    DataStorePaths {
        version: write(dir, ".version", VERSION),
        sampledata: write(dir, "sampledata.json", SAMPLEDATA_JSON),
        ec_numbers: write(dir, "ec_numbers.tsv", EC_NUMBERS_TSV),
        go_terms: write(dir, "go_terms.tsv", GO_TERMS_TSV),
        interpro_entries: write(dir, "interpro_entries.tsv", INTERPRO_ENTRIES_TSV),
        proteomes: write(dir, "proteomes.tsv", PROTEOMES_TSV),
        lineages: write(dir, "lineages.tsv", LINEAGES_TSV),
        taxons: write(dir, "taxons.tsv", TAXONS_TSV)
    }
}

/// Writes the protein corpus into `dir` and returns its path.
///
/// This is the input `sa-builder` reads; the index files built from it are produced separately, so
/// that nothing binary is ever committed.
pub fn write_proteins_tsv(dir: &Path) -> PathBuf {
    create_dir(dir);
    write(dir, "proteins.tsv", PROTEINS_TSV)
}

fn create_dir(dir: &Path) {
    fs::create_dir_all(dir).unwrap_or_else(|err| panic!("could not create {}: {}", dir.display(), err));
}

fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).unwrap_or_else(|err| panic!("could not write {}: {}", path.display(), err));
    path
}
