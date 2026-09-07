//! What each store does with input it cannot parse.
//!
//! The policy is uniform: a malformed row is an error, never a panic and never a silently dropped
//! line. Errors name the line they came from, because a store that refuses to load without saying
//! which row is worse to operate than one that quietly skipped it.

use datastore::{
    EcStore, EcStoreError, GoStore, GoStoreError, InterproStore, InterproStoreError, LineageStore, LineageStoreError,
    ReferenceProteomeStore, ReferenceProteomeStoreError, TaxonStore, TaxonStoreError
};
use tempfile::TempDir;

/// Writes `contents` to a file in a fresh temporary directory and hands the path to `load`.
///
/// The directory is dropped before returning, which is safe because every store reads its file
/// eagerly and keeps nothing open.
fn with_file<T, E>(name: &str, contents: &str, load: impl Fn(&str) -> Result<T, E>) -> Result<T, E> {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let path = dir.path().join(name);
    std::fs::write(&path, contents).expect("could not write the fixture file");
    load(path.to_str().unwrap())
}

/// Drops a successfully loaded store, keeping only whether it loaded.
///
/// The stores do not implement `Debug`, so a failure message can only render the error side.
fn outcome<T, E>(result: Result<T, E>) -> Result<(), E> {
    result.map(|_| ())
}

/// A well-formed lineage row: a taxon id followed by 28 unrecorded rank columns.
fn valid_row(taxon_id: u32) -> String {
    let ranks = ["\\N"; 28].join("\t");
    format!("{taxon_id}\t{ranks}")
}

fn load_lineages(contents: &str) -> (TempDir, Result<LineageStore, LineageStoreError>) {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let path = dir.path().join("lineages.tsv");
    std::fs::write(&path, contents).expect("could not write the lineage file");

    let result = LineageStore::try_from_file(&path.to_string_lossy());
    (dir, result)
}

/// Loads `contents` and returns the error it was rejected with.
fn rejection(contents: &str) -> LineageStoreError {
    let (_dir, result) = load_lineages(contents);
    match result {
        Ok(_) => panic!("expected the file to be rejected, but it loaded"),
        Err(error) => error
    }
}

#[test]
fn a_well_formed_file_still_loads() {
    let (_dir, result) = load_lineages(&format!("{}\n{}\n", valid_row(1), valid_row(2)));
    assert!(result.is_ok(), "the baseline row shape must parse, or the rest of this file proves nothing");
}

/// A short row is reported as short — including when its fields are also unparseable.
///
/// The width is checked before anything is parsed, so a row of nonsense that is also the wrong
/// length points at the length. Getting that order backwards sends the reader after the wrong
/// problem entirely.
#[test]
fn a_row_with_too_few_columns_is_an_error() {
    match rejection("8501\t2759\t\\N\n") {
        LineageStoreError::UnexpectedColumnCount { line, expected, found } => {
            assert_eq!(line, 1);
            assert_eq!(expected, 29);
            assert_eq!(found, 3);
        }
        other => panic!("expected an UnexpectedColumnCount error, got {other:?}")
    }

    let nonsense = rejection("nonsense\tmore nonsense\n");
    assert!(matches!(nonsense, LineageStoreError::UnexpectedColumnCount { .. }), "got {nonsense:?}");
}

#[test]
fn a_row_with_too_many_columns_is_an_error() {
    match rejection(&format!("{}\t9999\n", valid_row(8501))) {
        LineageStoreError::UnexpectedColumnCount { found, .. } => assert_eq!(found, 30),
        other => panic!("expected an UnexpectedColumnCount error, got {other:?}")
    }
}

#[test]
fn a_non_numeric_taxon_id_is_an_error() {
    let ranks = ["\\N"; 28].join("\t");

    match rejection(&format!("crocodile\t{ranks}\n")) {
        LineageStoreError::InvalidTaxonId { line, value } => {
            assert_eq!(line, 1);
            assert_eq!(value, "crocodile");
        }
        other => panic!("expected an InvalidTaxonId error, got {other:?}")
    }
}

#[test]
fn a_non_numeric_rank_id_is_an_error() {
    let mut fields = vec!["8501".to_string()];
    fields.extend(std::iter::repeat_n("\\N".to_string(), 28));
    fields[9] = "not-a-taxon".to_string();

    match rejection(&fields.join("\t")) {
        LineageStoreError::InvalidRankId { column, value, .. } => {
            assert_eq!(value, "not-a-taxon");
            // fields[9] is the tenth column of the row, counting the taxon id as the first.
            assert_eq!(column, 10);
        }
        other => panic!("expected an InvalidRankId error, got {other:?}")
    }
}

#[test]
fn the_error_names_the_line_it_came_from() {
    let error = rejection(&format!("{}\n{}\n8501\tbroken\n", valid_row(1), valid_row(2)));
    assert!(error.to_string().starts_with("Line 3:"), "got {error}");
}

/// A row of nothing but delimiters is malformed input, not a blank line.
///
/// It is the case a `trim()` before the emptiness check silently swallows: 28 tabs trim to nothing
/// and the row disappears, which is the behaviour this policy exists to remove.
#[test]
fn a_row_of_only_delimiters_is_an_error() {
    // 28 tabs is 29 fields, so this row is the right width and fails on its empty taxon id. A
    // shorter run of tabs fails on the width. Either way it is rejected rather than skipped, which
    // is the property that matters.
    assert!(matches!(rejection(&"\t".repeat(28)), LineageStoreError::InvalidTaxonId { .. }));
    assert!(matches!(rejection("\t\t\t"), LineageStoreError::UnexpectedColumnCount { .. }));
}

/// A blank line is not a malformed row. `lines()` yields one for a file that ends in two newlines,
/// and refusing to load such a file would turn an invisible whitespace difference into an outage.
///
/// `lines()` also strips a CRLF pair, so a Windows-authored file needs no separate handling: the
/// blank line arrives as `""` rather than `"\r"`, and no field carries a trailing `\r`.
#[test]
fn blank_lines_are_skipped() {
    let contents = format!("{}\r\n\r\n{}\n\n", valid_row(1), valid_row(8501));
    let (_dir, result) = load_lineages(&contents);

    let store = result.expect("blank lines should be skipped, not rejected");
    assert!(store.get(1).is_some());
    assert!(store.get(8501).is_some());
}

// ── The stores that used to drop a row they could not read ──────────────────────────────────────
//
// Each of these five skipped any line whose column count was wrong, so a truncated file loaded
// clean and simply held less than it should. The tests below pin the replacement behaviour, and
// `a_dropped_row_is_now_an_error` states the change itself rather than leaving it implicit.

/// A valid taxon row. The fifth column is the MySQL boolean the loader reads: 0x01 valid, 0x00 not.
fn taxon_row(id: u32, name: &str, rank: &str, valid: bool) -> String {
    let flag = if valid { '\u{1}' } else { '\u{0}' };
    format!("{id}\t{name}\t{rank}\t1\t{flag}")
}

#[test]
fn a_dropped_row_is_now_an_error() {
    // Before this policy each of these files loaded successfully, holding one entry instead of two.
    // Silently. That is the behaviour being replaced.
    let contents = format!("{}\n8501\tshort row\n", taxon_row(1, "root", "no rank", true));
    let taxons = outcome(with_file("taxons.tsv", &contents, TaxonStore::try_from_file));
    assert!(matches!(taxons, Err(TaxonStoreError::UnexpectedColumnCount { line: 2, .. })), "taxons: {taxons:?}");

    let ec = with_file("ec.tsv", "1\t1.1.1.1\tAlcohol dehydrogenase\n2\tbroken\n", EcStore::try_from_file);
    assert!(matches!(ec, Err(EcStoreError::UnexpectedColumnCount { line: 2, .. })), "ec");

    let go = with_file("go.tsv", "1\tGO:1\tns\tname\n2\tbroken\n", GoStore::try_from_file);
    assert!(matches!(go, Err(GoStoreError::UnexpectedColumnCount { line: 2, .. })), "go");

    let interpro = with_file("ipr.tsv", "1\tIPR1\tFamily\tname\n2\tbroken\n", InterproStore::try_from_file);
    assert!(matches!(interpro, Err(InterproStoreError::UnexpectedColumnCount { line: 2, .. })), "interpro");

    let proteomes =
        with_file("proteomes.tsv", "1\tUP1\t8501\t1\tP1\n2\tbroken\n", ReferenceProteomeStore::try_from_file);
    assert!(matches!(proteomes, Err(ReferenceProteomeStoreError::UnexpectedColumnCount { line: 2, .. })), "proteomes");
}

#[test]
fn taxon_store_rejects_an_unparseable_id_and_rank() {
    let bad_id = outcome(with_file("taxons.tsv", "crocodile\tname\tspecies\t1\t\u{1}\n", TaxonStore::try_from_file));
    match bad_id {
        Err(TaxonStoreError::InvalidTaxonId { line, value }) => {
            assert_eq!(line, 1);
            assert_eq!(value, "crocodile");
        }
        other => panic!("expected InvalidTaxonId, got {other:?}")
    }

    let bad_rank =
        outcome(with_file("taxons.tsv", &taxon_row(1, "root", "not a rank", true), TaxonStore::try_from_file));
    match bad_rank {
        Err(TaxonStoreError::InvalidRank { value, .. }) => assert_eq!(value, "not a rank"),
        other => panic!("expected InvalidRank, got {other:?}")
    }
}

/// The validity byte is 0x00 or 0x01, and neither is whitespace — which matters because the loader
/// trims the line before splitting it. If that ever became a space, `trim_end` would eat the column
/// and every row in the file would be one field short.
#[test]
fn taxon_store_keeps_the_validity_column_through_trimming() {
    let contents = format!(
        "{}\n{}\n",
        taxon_row(1, "valid taxon", "species", true),
        taxon_row(2, "invalid taxon", "species", false)
    );
    let store = with_file("taxons.tsv", &contents, TaxonStore::try_from_file).expect("both rows should parse");

    assert!(store.is_valid(1));
    assert!(!store.is_valid(2));
    assert!(store.get(2).is_some(), "an invalid taxon is still stored, just flagged");
}

#[test]
fn blank_lines_are_skipped_by_every_store() {
    let contents = format!("\n{}\n\n", taxon_row(1, "root", "no rank", true));
    let taxons = with_file("taxons.tsv", &contents, TaxonStore::try_from_file);
    assert!(taxons.expect("blank lines should be skipped").get(1).is_some());

    assert!(with_file("ec.tsv", "\n1\t1.1.1.1\tname\n\n", EcStore::try_from_file).is_ok());
    assert!(with_file("go.tsv", "\n1\tGO:1\tns\tname\n\n", GoStore::try_from_file).is_ok());
    assert!(with_file("ipr.tsv", "\n1\tIPR1\tFamily\tname\n\n", InterproStore::try_from_file).is_ok());
    assert!(with_file("proteomes.tsv", "\n1\tUP1\t8501\t1\tP1\n\n", ReferenceProteomeStore::try_from_file).is_ok());
}

#[test]
fn reference_proteome_parse_errors_name_their_line() {
    let contents = "1\tUP1\t8501\t1\tP1\n2\tUP2\tnot-a-taxon\t1\tP2\n";
    let result = outcome(with_file("proteomes.tsv", contents, ReferenceProteomeStore::try_from_file));

    match result {
        Err(error @ ReferenceProteomeStoreError::ParseError(_)) => {
            assert!(error.to_string().contains("Line 2"), "got {error}");
        }
        other => panic!("expected a ParseError, got {other:?}")
    }
}

/// A row of delimiters is a row, not a blank line.
///
/// This is the case a `trim()` before the emptiness check swallowed: tabs trim to nothing, the row
/// vanished, and the silent drop this policy exists to remove came back. Each loader keeps its own
/// copy of the carve-out, so each is checked.
///
/// The rows below are one delimiter short of each store's width, so they fail the width check. A
/// delimiter row of the *right* width is a separate matter: `taxons` and `proteomes` still reject
/// it because their numeric columns are empty, while `ec`, `go` and `interpro` parse no fields and
/// accept it as a row with an empty key.
#[test]
fn a_row_of_only_delimiters_is_rejected_by_every_store() {
    let taxons = outcome(with_file("taxons.tsv", "\t\t\t\n", TaxonStore::try_from_file));
    assert!(matches!(taxons, Err(TaxonStoreError::UnexpectedColumnCount { found: 4, .. })), "taxons: {taxons:?}");

    let ec = outcome(with_file("ec.tsv", "\t\n", EcStore::try_from_file));
    assert!(matches!(ec, Err(EcStoreError::UnexpectedColumnCount { found: 2, .. })), "ec: {ec:?}");

    let go = outcome(with_file("go.tsv", "\t\t\n", GoStore::try_from_file));
    assert!(matches!(go, Err(GoStoreError::UnexpectedColumnCount { found: 3, .. })), "go: {go:?}");

    let ipr = outcome(with_file("ipr.tsv", "\t\t\n", InterproStore::try_from_file));
    assert!(matches!(ipr, Err(InterproStoreError::UnexpectedColumnCount { found: 3, .. })), "interpro: {ipr:?}");

    let prot = outcome(with_file("proteomes.tsv", "\t\t\t\n", ReferenceProteomeStore::try_from_file));
    assert!(matches!(prot, Err(ReferenceProteomeStoreError::UnexpectedColumnCount { found: 4, .. })), "{prot:?}");
}

/// A trailing delimiter is a sixth, empty column — not whitespace to be trimmed away.
///
/// The loader used to `trim_end()` the line before splitting it, and a tab is whitespace, so a row
/// with one delimiter too many arrived looking exactly like a well-formed one.
#[test]
fn a_trailing_delimiter_is_counted_as_a_column() {
    let row = format!("{}\t\n", taxon_row(1, "root", "no rank", true));
    let result = outcome(with_file("taxons.tsv", &row, TaxonStore::try_from_file));

    match result {
        Err(TaxonStoreError::UnexpectedColumnCount { found, .. }) => assert_eq!(found, 6),
        other => panic!("expected UnexpectedColumnCount with six columns, got {other:?}")
    }
}
