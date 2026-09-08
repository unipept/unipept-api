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
//! - **Taxonomy** — `data/taxons.tsv` and `data/lineages.tsv`: twenty-eight real NCBI taxa, copied
//!   verbatim out of a `taxons.tsv`/`lineages.tsv` pair from a Unipept database build. The set is
//!   closed under ancestry — every taxon a protein names, every ancestor those taxa's lineages
//!   record, and root — so it is small enough to read in full, which is what makes an expected LCA
//!   checkable by eye rather than by rerunning the code. The `melanogaster` pair carries the two
//!   multi-word ranks a lineage column spells with an underscore, and carries no protein.
//! - **Proteins** — `data/proteins.tsv`: thirteen rows referencing only taxa from that subset.
//!
//! To regenerate the taxonomy after changing the proteins: take the taxon column of
//! `proteins.tsv`, union it with every non-`\N` rank id on those taxa's rows in the build's
//! lineage table, add taxon 1, and keep the `melanogaster` pair, which no protein names. Copy the
//! matching rows out of both tables rather than rewriting them — the fifth taxon column is a raw
//! `0x01`/`0x00` byte, not text. Twenty-eight ancestors are named in a lineage column without
//! having a row of their own, which is a property of a sampled taxonomy and not an error.
//!
//! Writers panic rather than returning errors. A fixture that cannot be written to a temporary
//! directory is a broken harness, not a condition a test should handle; this is the opposite of
//! the policy for *parsing* corpus files, where malformed input must always surface as an error.

use std::{
    fs,
    path::{Path, PathBuf}
};

pub mod synthetic;

#[cfg(feature = "index-builder")]
mod index_builder;
#[cfg(feature = "index-builder")]
pub use index_builder::{
    IndexPaths, build_index_files, build_index_files_from, build_index_files_from_with_kmer_table,
    build_index_files_with_kmer_table
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

/// Lineages: a taxon id followed by one column per rank, `\N` where the taxonomy records nothing.
///
/// The column count is `LineageStore::AMOUNT_OF_RANKS`.
pub const LINEAGES_TSV: &str = include_str!("../data/lineages.tsv");

/// Every accession in the corpus, in file order.
///
/// Database mocks key their canned responses on these, so that an index built from the corpus and
/// a mocked OpenSearch agree about which proteins exist.
pub const ACCESSIONS: [&str; 13] = [
    "P00001", "P00002", "P00003", "P00004", "P00005", "P00006", "P00007", "P00008", "P00009", "P00010", "P00011",
    "P00012", "P00013"
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
    /// The `melanogaster group`, a taxon of rank `species group`.
    ///
    /// Its rank name holds a space, which the lineage columns write as an underscore. It carries no
    /// protein: nothing but a rank name is asked of it.
    pub const MELANOGASTER_GROUP: u32 = 32346;
    /// The `melanogaster subgroup`, of rank `species subgroup`, and the only descendant of
    /// [`MELANOGASTER_GROUP`].
    pub const MELANOGASTER_SUBGROUP: u32 = 32351;
    /// `Heloderma sp.`, and the only taxon here the taxonomy marks **invalid**.
    ///
    /// Without one, `TaxonStore::is_valid` has no counterexample and `calculate_lca`'s
    /// `only_valid_taxa` filter is a no-op against this corpus — so `validate_taxa`, a real
    /// `pept2lca` parameter, could not be exercised at all.
    pub const HELODERMA: u32 = 8553;
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
    /// Present in seven of the thirteen proteins, spanning both domains — enough occurrences for a
    /// low cutoff to bite and set `cutoff_used`.
    pub const COMMON: &str = "AAGGK";
    /// Shared by a protein of `C. niloticus` and one of the invalid `Heloderma sp.`
    ///
    /// The pair `validate_taxa` is visible through: with the filter off both taxa survive and the
    /// LCA is their common ancestor; with it on the invalid one is dropped and the LCA collapses to
    /// `C. niloticus` alone.
    pub const VALIDATION_SHARED: &str = "VALIDATEKR";
    /// In no protein. Distinguishes an empty result from an error.
    pub const ABSENT: &str = "WWWWWWWWWW";
}

/// How many corpus taxa carry a rank and are marked valid.
///
/// Counted from `TAXONS_TSV` rather than written down, so adding a taxon cannot leave a test
/// asserting a stale total. Root is the only unranked row, [`taxa::HELODERMA`] the only invalid one.
pub fn ranked_and_valid_taxa() -> u64 {
    TAXONS_TSV
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| {
            let mut columns = line.split('\t').skip(2);
            columns.next() != Some("no rank") && columns.nth(1) == Some("\u{1}")
        })
        .count() as u64
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// `(accession, taxon, sequence)` for every row of the protein corpus.
    fn proteins() -> Vec<(&'static str, u32, &'static str)> {
        PROTEINS_TSV
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let mut fields = line.split('\t');
                let accession = fields.next().expect("a protein row starts with an accession");
                let taxon = fields.next().expect("a protein row has a taxon column");
                let sequence = fields.next().expect("a protein row has a sequence column");
                (accession, taxon.parse().expect("the taxon column is numeric"), sequence)
            })
            .collect()
    }

    /// The taxon id of every row in a taxonomy table.
    fn row_ids(tsv: &'static str) -> BTreeSet<u32> {
        tsv.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.split('\t').next().unwrap().parse().expect("a taxonomy row starts with a taxon id"))
            .collect()
    }

    /// Which taxa each peptide's proteins belong to.
    fn taxa_containing(peptide: &str) -> BTreeSet<u32> {
        proteins().iter().filter(|(_, _, seq)| seq.contains(peptide)).map(|(_, taxon, _)| *taxon).collect()
    }

    /// `ACCESSIONS` is written out by hand beside the file it describes, so nothing but this stops
    /// the two drifting when a protein is added.
    #[test]
    fn accessions_match_the_protein_corpus() {
        let from_file: Vec<&str> = proteins().iter().map(|(accession, _, _)| *accession).collect();
        assert_eq!(ACCESSIONS.to_vec(), from_file);
    }

    #[test]
    fn every_named_taxon_has_a_row_in_the_taxon_table() {
        let ids = row_ids(TAXONS_TSV);
        for taxon in [
            taxa::ROOT,
            taxa::AZORHIZOBIUM_CAULINODANS,
            taxa::BUCHNERA_APHIDICOLA,
            taxa::SARCOPTERYGII,
            taxa::CROCODYLIDAE,
            taxa::CROCODYLUS,
            taxa::CROCODYLUS_NILOTICUS,
            taxa::CROCODYLUS_POROSUS,
            taxa::CROCODYLUS_NOVAEGUINEAE,
            taxa::SPHENODONTIA,
            taxa::ALOUATTA_SENICULUS,
            taxa::MELANOGASTER_GROUP,
            taxa::MELANOGASTER_SUBGROUP,
            taxa::HELODERMA
        ] {
            assert!(ids.contains(&taxon), "taxa:: names {taxon}, which has no row in taxons.tsv");
        }
    }

    /// The invariant the whole corpus rests on. If the protein and taxonomy halves stop agreeing,
    /// searches return taxa that cannot be named and the suite goes vacuous rather than red.
    #[test]
    fn every_protein_taxon_has_both_a_taxon_and_a_lineage_row() {
        let taxons = row_ids(TAXONS_TSV);
        let lineages = row_ids(LINEAGES_TSV);

        for (accession, taxon, _) in proteins() {
            assert!(taxons.contains(&taxon), "{accession} names taxon {taxon}, absent from taxons.tsv");
            assert!(lineages.contains(&taxon), "{accession} names taxon {taxon}, absent from lineages.tsv");
        }
    }

    /// One taxon at each of the two multi-word ranks a lineage column spells with an underscore.
    #[test]
    fn the_corpus_holds_a_taxon_at_each_multi_word_rank() {
        let ranks: BTreeSet<&str> = TAXONS_TSV
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.split('\t').nth(2).expect("a taxon row has a rank column"))
            .collect();

        assert!(ranks.contains("species group"), "{ranks:?}");
        assert!(ranks.contains("species subgroup"), "{ranks:?}");
    }

    #[test]
    fn the_two_taxonomy_tables_describe_the_same_taxa() {
        assert_eq!(row_ids(TAXONS_TSV), row_ids(LINEAGES_TSV));
    }

    /// Each marker peptide's documentation states which taxa it reaches, and those claims are the
    /// corpus's actual specification — a peptide that stops reaching two species silently turns the
    /// genus scenario into the species scenario without any test mentioning peptides failing.
    #[test]
    fn marker_peptides_reach_the_taxa_their_documentation_claims() {
        use taxa::*;

        assert_eq!(taxa_containing(peptides::UNIQUE), BTreeSet::from([CROCODYLUS_NILOTICUS]));
        assert_eq!(taxa_containing(peptides::GENUS_SHARED), BTreeSet::from([CROCODYLUS_NILOTICUS, CROCODYLUS_POROSUS]));
        assert_eq!(
            taxa_containing(peptides::SUPERCLASS_SHARED),
            BTreeSet::from([CROCODYLUS_NILOTICUS, ALOUATTA_SENICULUS])
        );
        assert_eq!(
            taxa_containing(peptides::ROOT_SHARED),
            BTreeSet::from([CROCODYLUS_NILOTICUS, AZORHIZOBIUM_CAULINODANS])
        );
        assert_eq!(taxa_containing(peptides::IL_ISOLEUCINE), BTreeSet::from([CROCODYLUS_POROSUS]));
        assert_eq!(taxa_containing(peptides::IL_LEUCINE), BTreeSet::from([CROCODYLUS_NOVAEGUINEAE]));
        assert_eq!(taxa_containing(peptides::VALIDATION_SHARED), BTreeSet::from([CROCODYLUS_NILOTICUS, HELODERMA]));
        assert!(taxa_containing(peptides::ABSENT).is_empty());

        // The doc comment claims seven of the twelve proteins, which is the count a low cutoff has
        // to bite against — the taxa alone would not catch a protein being dropped.
        let common = proteins().iter().filter(|(_, _, seq)| seq.contains(peptides::COMMON)).count();
        assert_eq!(common, 7);
    }

    /// Exactly one taxon is marked invalid, and it is the one the validation marker reaches.
    ///
    /// The fifth column is a raw byte, so this also fails if a copy through a text editor ever
    /// turns it into something printable.
    #[test]
    fn the_corpus_contains_exactly_one_invalid_taxon() {
        let invalid: Vec<u32> = TAXONS_TSV
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| line.split('\t').nth(4) != Some("\u{1}"))
            .map(|line| line.split('\t').next().unwrap().parse().unwrap())
            .collect();

        assert_eq!(invalid, vec![taxa::HELODERMA]);
    }

    /// The I/L pair must differ at exactly one position, or `equate_il` proves nothing.
    #[test]
    fn the_isoleucine_and_leucine_markers_differ_only_in_i_and_l() {
        let differences = peptides::IL_ISOLEUCINE
            .chars()
            .zip(peptides::IL_LEUCINE.chars())
            .filter(|(left, right)| left != right)
            .collect::<Vec<_>>();

        assert_eq!(differences, vec![('I', 'L')]);
    }
}
