//! Searching an index built from the shared corpus.
//!
//! The index files are built here rather than committed. Building costs one `libsais` run over a
//! few hundred residues, and it means the fixture can never be a stale artifact of a different
//! `unipept-index` than the one the lockfile resolves.

use std::collections::HashSet;

use fixtures::peptides::*;
use index::{Index, IndexError, LoadIndexError};
use tempfile::TempDir;

/// Builds the corpus index. The `TempDir` is returned because dropping it removes the files, and
/// the memory-mapped backends keep reading them for as long as the index is alive.
fn corpus_index() -> (TempDir, Index) {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let paths = fixtures::build_index_files(dir.path());

    // Several tests here are about the no-table fallback, and none of them would fail if the
    // builder quietly started writing one — they would just stop covering the path they name.
    assert!(!paths.kmer_table.exists(), "the fixture builder must not write a k-mer table");

    let index = Index::try_from_files(
        paths.suffix_array.to_str().unwrap(),
        paths.proteins.to_str().unwrap(),
        paths.mapping.to_str().unwrap(),
        paths.kmer_table.to_str().unwrap()
    )
    .expect("the corpus index should load");

    (dir, index)
}

fn search(index: &Index, peptide: &str, equate_il: bool) -> Vec<(String, u32)> {
    let peptides = vec![peptide.to_string()];
    let mut hits: Vec<(String, u32)> = index
        .analyse(&peptides, equate_il, false, None)
        .into_iter()
        .flat_map(|result| {
            result.proteins.into_iter().map(|protein| (protein.uniprot_accession.to_string(), protein.taxon))
        })
        .collect();
    hits.sort();
    hits
}

/// The index loads with no k-mer table at all.
///
/// `try_from_files` treats the table as an accelerator and falls back to searching the whole suffix
/// array, which is the configuration every test in this file runs in.
#[test]
fn the_index_loads_without_a_kmer_table() {
    let (_dir, index) = corpus_index();
    assert_eq!(search(&index, UNIQUE, false).len(), 1);
}

#[test]
fn a_unique_peptide_resolves_to_one_protein() {
    let (_dir, index) = corpus_index();
    assert_eq!(search(&index, UNIQUE, false), vec![("P00001".to_string(), fixtures::taxa::CROCODYLUS_NILOTICUS)]);
}

#[test]
fn a_peptide_shared_within_a_genus_finds_both_species() {
    let (_dir, index) = corpus_index();

    assert_eq!(search(&index, GENUS_SHARED, false), vec![
        ("P00001".to_string(), fixtures::taxa::CROCODYLUS_NILOTICUS),
        ("P00003".to_string(), fixtures::taxa::CROCODYLUS_POROSUS)
    ]);
}

#[test]
fn a_peptide_shared_across_domains_finds_both() {
    let (_dir, index) = corpus_index();

    assert_eq!(search(&index, ROOT_SHARED, false), vec![
        ("P00001".to_string(), fixtures::taxa::CROCODYLUS_NILOTICUS),
        ("P00006".to_string(), fixtures::taxa::AZORHIZOBIUM_CAULINODANS)
    ]);
}

/// A peptide that matches nothing produces **no result row**, rather than a row with an empty
/// protein list. Callers that zip inputs to outputs positionally would be wrong about which peptide
/// each result belongs to, so this is worth stating rather than discovering.
#[test]
fn an_absent_peptide_produces_no_result_row_at_all() {
    let (_dir, index) = corpus_index();
    let peptides = vec![ABSENT.to_string(), UNIQUE.to_string()];

    let results = index.analyse(&peptides, false, false, None);

    assert_eq!(results.len(), 1, "only the peptide that matched should appear");
    assert_eq!(results[0].sequence, UNIQUE);
}

/// `equate_il` is the one flag with a directly observable effect on this corpus: two proteins whose
/// peptides differ only at an I/L position.
#[test]
fn equate_il_merges_an_isoleucine_and_leucine_pair() {
    let (_dir, index) = corpus_index();

    // Apart, each peptide finds only its own species.
    assert_eq!(search(&index, IL_ISOLEUCINE, false), vec![("P00003".to_string(), fixtures::taxa::CROCODYLUS_POROSUS)]);
    assert_eq!(search(&index, IL_LEUCINE, false), vec![(
        "P00004".to_string(),
        fixtures::taxa::CROCODYLUS_NOVAEGUINEAE
    )]);

    // Equated, both find both — and, being two species of one genus, they now reduce to the genus.
    let equated = search(&index, IL_ISOLEUCINE, true);
    assert_eq!(equated, search(&index, IL_LEUCINE, true), "the two spellings must become interchangeable");
    assert_eq!(equated, vec![
        ("P00003".to_string(), fixtures::taxa::CROCODYLUS_POROSUS),
        ("P00004".to_string(), fixtures::taxa::CROCODYLUS_NOVAEGUINEAE)
    ]);
}

/// A metamorphic property rather than a fixed expectation: whatever the tryptic rules are, a
/// tryptic search can only ever return a subset of the same search without them.
#[test]
fn tryptic_results_are_a_subset_of_untryptic_results() {
    let (_dir, index) = corpus_index();
    let peptides: Vec<String> = [UNIQUE, GENUS_SHARED, SUPERCLASS_SHARED, ROOT_SHARED, COMMON]
        .iter()
        .map(|peptide| peptide.to_string())
        .collect();

    let all: HashSet<(String, String)> = accession_pairs(&index, &peptides, false);
    let tryptic: HashSet<(String, String)> = accession_pairs(&index, &peptides, true);

    assert!(!tryptic.is_empty(), "the corpus must contain at least one tryptic match, or this proves nothing");
    assert!(tryptic.is_subset(&all), "tryptic matches not present in the unfiltered result: {:?}", &tryptic - &all);
}

fn accession_pairs(index: &Index, peptides: &[String], tryptic: bool) -> HashSet<(String, String)> {
    index
        .analyse(peptides, false, tryptic, None)
        .into_iter()
        .flat_map(|result| {
            let sequence = result.sequence.to_string();
            result
                .proteins
                .into_iter()
                .map(move |protein| (sequence.clone(), protein.uniprot_accession.to_string()))
        })
        .collect()
}

/// Below the cutoff nothing changes; above it the result is a truncated sample and says so.
#[test]
fn a_cutoff_truncates_the_matches_and_flags_it() {
    let (_dir, index) = corpus_index();
    let peptides = vec![COMMON.to_string(), UNIQUE.to_string()];

    let uncapped = index.analyse_taxa(&peptides, false, false, None);
    let common = uncapped.iter().find(|result| result.sequence == COMMON).expect("the common peptide matches");
    assert!(!common.cutoff_used);
    assert_eq!(common.taxa.len(), 6, "the common peptide spans six taxa");

    let capped = index.analyse_taxa(&peptides, false, false, Some(2));
    let common = capped.iter().find(|result| result.sequence == COMMON).expect("the common peptide matches");
    let unique = capped.iter().find(|result| result.sequence == UNIQUE).expect("the unique peptide matches");

    assert!(common.cutoff_used, "seven matching proteins against a cutoff of two must set the flag");
    assert!(!unique.cutoff_used, "a single match is below any cutoff");
}

/// `analyse_taxa` is the cheap path `pept2lca` takes: it never retrieves an accession or an
/// annotation. Substituting it for `analyse` is only sound if it reports the same taxa.
#[test]
fn analyse_taxa_agrees_with_analyse() {
    let (_dir, index) = corpus_index();
    let peptides: Vec<String> = [UNIQUE, GENUS_SHARED, SUPERCLASS_SHARED, ROOT_SHARED, COMMON]
        .iter()
        .map(|p| p.to_string())
        .collect();

    for result in index.analyse_taxa(&peptides, false, false, None) {
        let from_proteins: Vec<u32> = index
            .analyse(&[result.sequence.to_string()], false, false, None)
            .into_iter()
            .flat_map(|full| full.proteins.into_iter().map(|protein| protein.taxon))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        assert_eq!(result.taxa, from_proteins, "taxa disagree for {}", result.sequence);
        assert!(result.taxa.windows(2).all(|pair| pair[0] < pair[1]), "taxa must be sorted and deduplicated");
    }
}

/// The one check that the four index files came from the same build.
///
/// Without it a stale mapping loads, the index reports ready, and every search attributes peptides
/// to the wrong proteins — the failure mode that makes committing built index files a bad trade.
#[test]
fn a_mapping_from_a_different_index_is_rejected() {
    let corpus = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();

    let corpus_paths = fixtures::build_index_files(corpus.path());
    let other_paths = fixtures::build_index_files_from(
        other.path(),
        "Q00001\t8501\tMKWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWQ\t\nQ00002\t8502\tMYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYQ\t\n"
    );

    let mixed = Index::try_from_files(
        corpus_paths.suffix_array.to_str().unwrap(),
        corpus_paths.proteins.to_str().unwrap(),
        other_paths.mapping.to_str().unwrap(),
        corpus_paths.kmer_table.to_str().unwrap()
    );

    match mixed {
        Err(error) => assert!(
            matches!(error, IndexError::LoadError(LoadIndexError::MismatchedIndexFiles(_))),
            "the files must be rejected as mismatched, not merely fail to load: {error:?}"
        ),
        Ok(_) => panic!("a mapping built from different proteins must not load")
    }
}

/// Every accession the index hands back is one the corpus declares.
///
/// Not the converse: the corpus holds thirteen proteins and only seven carry the common peptide,
/// so this cannot and does not prove the index contains all of them. What it rules out is the
/// index returning an accession `ACCESSIONS` has never heard of — which is what would happen if
/// the index and the database mocks drifted apart and a composed `AppState` started answering
/// with proteins the mocked database cannot resolve.
#[test]
fn every_accession_the_index_returns_is_declared_by_the_corpus() {
    let (_dir, index) = corpus_index();
    let peptides = vec![COMMON.to_string()];

    let found: HashSet<String> = index
        .analyse(&peptides, false, false, None)
        .into_iter()
        .flat_map(|result| result.proteins.into_iter().map(|p| p.uniprot_accession.to_string()))
        .collect();

    for accession in &found {
        assert!(
            fixtures::ACCESSIONS.contains(&accession.as_str()),
            "{accession} is in the index but not in ACCESSIONS"
        );
    }
}

// ── The k-mer table ─────────────────────────────────────────────────────────────────────────────
//
// Everything above runs without one. That is the deployed-with-no-table configuration and the
// fallback `try_from_files` documents, but it left the whole loading branch — and the cross-file
// check that guards it — unreached.

/// `k = 3` keeps the table to a few hundred kilobytes; the production default of 5 is over a
/// hundred megabytes, since the size follows the alphabet rather than the corpus.
const FIXTURE_KMER_SIZE: usize = 3;

fn index_from(paths: &fixtures::IndexPaths) -> Result<Index, IndexError> {
    Index::try_from_files(
        paths.suffix_array.to_str().unwrap(),
        paths.proteins.to_str().unwrap(),
        paths.mapping.to_str().unwrap(),
        paths.kmer_table.to_str().unwrap()
    )
}

#[test]
fn an_index_loads_with_a_kmer_table() {
    let dir = TempDir::new().unwrap();
    let paths = fixtures::build_index_files_with_kmer_table(dir.path(), FIXTURE_KMER_SIZE);

    assert!(paths.kmer_table.exists(), "this builder must write the table");
    let index = index_from(&paths).expect("an index with a table should load");

    assert_eq!(search(&index, UNIQUE, false), vec![("P00001".to_string(), fixtures::taxa::CROCODYLUS_NILOTICUS)]);
}

/// The table is an accelerator, so it must not change a single answer.
///
/// A differential rather than a fixed expectation: whatever the searches return, the two builds
/// have to agree, and a table that narrows the wrong bounds would show up here as a missing hit
/// rather than as an error.
#[test]
fn a_kmer_table_changes_no_results() {
    let plain_dir = TempDir::new().unwrap();
    let table_dir = TempDir::new().unwrap();
    let plain = index_from(&fixtures::build_index_files(plain_dir.path())).expect("loads");
    let accelerated =
        index_from(&fixtures::build_index_files_with_kmer_table(table_dir.path(), FIXTURE_KMER_SIZE)).expect("loads");

    for peptide in [UNIQUE, GENUS_SHARED, SUPERCLASS_SHARED, ROOT_SHARED, IL_ISOLEUCINE, COMMON, ABSENT] {
        for equate_il in [false, true] {
            assert_eq!(
                search(&plain, peptide, equate_il),
                search(&accelerated, peptide, equate_il),
                "{peptide} disagrees between the plain and accelerated builds (equate_il={equate_il})"
            );
        }
    }
}

/// A table built over a larger text is refused, because its bounds run past this suffix array.
#[test]
fn a_kmer_table_from_a_larger_index_is_rejected() {
    let corpus_dir = TempDir::new().unwrap();
    let larger_dir = TempDir::new().unwrap();

    let corpus = fixtures::build_index_files(corpus_dir.path());

    // Enough extra protein to push the suffix array well past the corpus's, so the table's bounds
    // cannot fit inside it.
    let mut larger_tsv = String::from(fixtures::PROTEINS_TSV);
    for n in 0..40 {
        let filler = "AGKTNDSVEQ".repeat(n % 5 + 1);
        larger_tsv.push_str(&format!("Z{n:05}\t8501\tMKWQNPFSAHTVGEDLYCR{filler}\t\n"));
    }
    let larger = fixtures::build_index_files_from_with_kmer_table(larger_dir.path(), &larger_tsv, FIXTURE_KMER_SIZE);

    let mixed = Index::try_from_files(
        corpus.suffix_array.to_str().unwrap(),
        corpus.proteins.to_str().unwrap(),
        corpus.mapping.to_str().unwrap(),
        larger.kmer_table.to_str().unwrap()
    );

    match mixed {
        Err(error) => assert!(
            matches!(error, IndexError::LoadError(LoadIndexError::MismatchedIndexFiles(_))),
            "expected the table to be rejected as mismatched, got {error:?}"
        ),
        Ok(_) => panic!("a k-mer table built over a larger text must not load")
    }
}

/// The line the server prints at startup, and the only way to tell which backend a binary was
/// compiled for. `README.md` quotes its shape, so it is worth one assertion that it keeps it.
#[test]
fn the_backend_summary_names_each_structure() {
    let summary = Index::backend_summary();

    for structure in ["sa=", "text=", "proteins=", "mapping="] {
        assert!(summary.contains(structure), "`{structure}` missing from `{summary}`");
    }
}
