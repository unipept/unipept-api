//! The index a UniProt version is served from, named as unipept-database's `load.sh` names it.

use database::{DatabaseError, index_name};

#[test]
fn a_version_names_its_index_with_dashes() {
    assert_eq!(index_name("2026.03").unwrap(), "uniprot_entries-2026-03");
}

/// `.version` ends with a newline, and the datastore keeps whatever it does not trim.
#[test]
fn surrounding_whitespace_is_not_part_of_the_version() {
    assert_eq!(index_name(" 2026.03\n").unwrap(), "uniprot_entries-2026-03");
}

/// The test fixtures carry a suffix, and a suffixed build is still a valid name.
#[test]
fn a_suffixed_version_keeps_its_suffix() {
    assert_eq!(index_name("2026.09-fixtures").unwrap(), "uniprot_entries-2026-09-fixtures");
}

/// Each would make a name OpenSearch refuses, or one that reaches another path, so the process
/// stops at startup instead of failing every protein query.
#[test]
fn a_version_that_makes_no_valid_index_name_is_refused() {
    for version in ["", "  ", "2026/03", "../2026.03", "2026.03 x", "Release", "-2026.03", "2026.03*"] {
        assert!(matches!(index_name(version), Err(DatabaseError::InvalidVersion(_))), "{version:?} should be refused");
    }
}
