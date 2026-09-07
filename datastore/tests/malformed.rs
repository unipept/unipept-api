//! What each store does with input it cannot parse.
//!
//! The policy is uniform: a malformed row is an error, never a panic and never a silently dropped
//! line. Errors name the line they came from, because a store that refuses to load without saying
//! which row is worse to operate than one that quietly skipped it.

use datastore::{LineageStore, LineageStoreError};
use tempfile::TempDir;

/// A well-formed lineage row: a taxon id followed by 28 unrecorded rank columns.
fn valid_row(taxon_id: u32) -> String {
    let ranks = ["\\N"; 28].join("\t");
    format!("{taxon_id}\t{ranks}")
}

fn load_lineages(contents: &str) -> (TempDir, Result<LineageStore, LineageStoreError>) {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let path = dir.path().join("lineages.tsv");
    std::fs::write(&path, contents).expect("could not write the lineage file");

    let result = LineageStore::try_from_file(path.to_str().unwrap());
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

/// The width check runs before any field is parsed, so a short row full of nonsense is reported as
/// a short row. Getting this backwards produces an error that sends the reader after the wrong
/// problem entirely.
#[test]
fn a_short_row_is_reported_as_short_not_as_unparseable() {
    let error = rejection("nonsense\tmore nonsense\n");
    assert!(matches!(error, LineageStoreError::UnexpectedColumnCount { .. }), "got {error:?}");
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
