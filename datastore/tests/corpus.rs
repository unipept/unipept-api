//! The eight stores, read from the shared fixture corpus.
//!
//! These cover the path that works. The malformed-input behaviour of each parser is a separate
//! concern and lives beside these, so that a change to the error policy cannot quietly alter what
//! well-formed input parses to.

use datastore::{DataStore, TaxonRank};
use tempfile::TempDir;

/// Loads the whole corpus from a temporary directory.
///
/// All eight files are written there, the taxonomy included, so the `TempDir` really does own
/// everything the store was built from — and it is returned because dropping it deletes them.
fn load() -> (TempDir, DataStore) {
    let dir = TempDir::new().expect("could not create a temporary directory");
    let paths = fixtures::write_datastore_files(dir.path());

    let store = DataStore::try_from_files(
        paths.version.to_str().unwrap(),
        paths.sampledata.to_str().unwrap(),
        paths.ec_numbers.to_str().unwrap(),
        paths.go_terms.to_str().unwrap(),
        paths.interpro_entries.to_str().unwrap(),
        paths.proteomes.to_str().unwrap(),
        paths.lineages.to_str().unwrap(),
        paths.taxons.to_str().unwrap()
    )
    .expect("the fixture corpus should load");

    (dir, store)
}

#[test]
fn version_is_read_and_trimmed() {
    let (_dir, store) = load();
    assert_eq!(store.version(), "2026.09-fixtures");
}

#[test]
fn taxon_store_reads_name_rank_and_validity() {
    let (_dir, store) = load();
    let taxons = store.taxon_store();

    let (name, rank, valid) = taxons.get(fixtures::taxa::CROCODYLUS_NILOTICUS).expect("8501 is in the corpus");
    assert_eq!(name, "Crocodylus niloticus");
    assert_eq!(*rank, TaxonRank::SPECIES);
    assert!(valid);

    assert_eq!(taxons.get_name(fixtures::taxa::CROCODYLUS).map(String::as_str), Some("Crocodylus"));
    assert!(taxons.is_valid(fixtures::taxa::CROCODYLUS_NILOTICUS));
}

/// An invalid taxon is stored like any other and flagged, not dropped.
///
/// `calculate_lca` filters on this when `validate_taxa` is set, so a store that either lost the
/// row or read the flag backwards would turn that parameter into a no-op — silently, since the
/// only visible effect is an LCA that is one rank too deep.
#[test]
fn taxon_store_keeps_an_invalid_taxon_and_marks_it() {
    let (_dir, store) = load();
    let taxons = store.taxon_store();

    let (name, _, valid) = taxons.get(fixtures::taxa::HELODERMA).expect("the invalid taxon is still stored");
    assert_eq!(name, "Heloderma sp.");
    assert!(!valid);
    assert!(!taxons.is_valid(fixtures::taxa::HELODERMA));
    assert!(taxons.is_valid(fixtures::taxa::CROCODYLUS_NILOTICUS), "validity must not read as false for everything");
}

#[test]
fn taxon_store_reads_the_root_taxon() {
    let (_dir, store) = load();

    // "no rank" is the one rank string that is not simply the lowercased variant name, so the root
    // row is the one that catches a `FromStr` arm going missing.
    let (name, rank, _) = store.taxon_store().get(fixtures::taxa::ROOT).expect("root is in the corpus");
    assert_eq!(name, "root");
    assert_eq!(*rank, TaxonRank::NO_RANK);
}

#[test]
fn taxon_store_returns_none_for_an_unknown_taxon() {
    let (_dir, store) = load();
    assert!(store.taxon_store().get(999_999_999).is_none());
    assert!(!store.taxon_store().is_valid(999_999_999));
}

#[test]
fn lineage_store_records_an_unrecorded_rank_as_none() {
    let (_dir, store) = load();

    // Crocodylus niloticus has no class in this taxonomy. The gap is what LCA reduction has to
    // step over to reach the superclass, so it matters that it survives parsing as `None` rather
    // than as a zero that compares equal to another lineage's missing class.
    let lineage = store.lineage_store().get(fixtures::taxa::CROCODYLUS_NILOTICUS).expect("8501 has a lineage");

    assert_eq!(lineage.class, None);
    assert_eq!(lineage.domain, Some(2759));
    assert_eq!(lineage.superclass, Some(fixtures::taxa::SARCOPTERYGII as i32));
    assert_eq!(lineage.family, Some(fixtures::taxa::CROCODYLIDAE as i32));
    assert_eq!(lineage.genus, Some(fixtures::taxa::CROCODYLUS as i32));
    assert_eq!(lineage.species, Some(fixtures::taxa::CROCODYLUS_NILOTICUS as i32));
}

#[test]
fn lineage_store_resolves_a_lineage_that_diverges_at_class() {
    let (_dir, store) = load();

    // The other side of the superclass scenario: Alouatta records a class where the crocodile does
    // not, which is exactly why their common ancestor sits above it.
    let lineage = store.lineage_store().get(fixtures::taxa::ALOUATTA_SENICULUS).expect("9503 has a lineage");

    assert_eq!(lineage.class, Some(40674));
    assert_eq!(lineage.superclass, Some(fixtures::taxa::SARCOPTERYGII as i32));
    assert_eq!(lineage.species, Some(fixtures::taxa::ALOUATTA_SENICULUS as i32));
}

#[test]
fn ec_store_is_keyed_without_the_annotation_prefix() {
    let (_dir, store) = load();
    let ec = store.ec_store();

    // Annotations in the protein corpus read `EC:1.1.1.1`, but the caller strips the prefix before
    // the lookup. A store keyed the other way would return nothing and be indistinguishable from
    // an EC number that is simply unknown.
    assert_eq!(ec.get("1.1.1.1").map(String::as_str), Some("Alcohol dehydrogenase"));
    assert!(ec.get("EC:1.1.1.1").is_none());
    assert!(ec.get("9.9.9.9").is_none());
}

#[test]
fn go_store_is_keyed_with_the_annotation_prefix() {
    let (_dir, store) = load();
    let go = store.go_store();

    // GO is the exception: its keys keep the prefix that EC and InterPro drop.
    assert_eq!(go.get_domain("GO:0009279"), Some("cellular component"));
    assert_eq!(go.get_name("GO:0009279"), Some("cell outer membrane"));
    assert!(go.get("0009279").is_none());
}

#[test]
fn interpro_store_is_keyed_without_the_annotation_prefix() {
    let (_dir, store) = load();
    let interpro = store.interpro_store();

    assert_eq!(interpro.get_domain("IPR016364"), Some("Family"));
    assert_eq!(interpro.get_name("IPR016364"), Some("Alcohol dehydrogenase, zinc-type"));
    assert!(interpro.get("IPR:IPR016364").is_none());
}

#[test]
fn reference_proteome_store_splits_the_protein_list_on_semicolons() {
    let (_dir, store) = load();
    let proteomes = store.reference_proteome_store();

    assert_eq!(proteomes.get_taxon_id("UP000000001"), Some(fixtures::taxa::CROCODYLUS_NILOTICUS));
    assert_eq!(proteomes.get_protein_count("UP000000001"), Some(3));
    assert_eq!(proteomes.get_proteins("UP000000001"), Some(vec!["P00001", "P00002", "P00010"]));
    assert!(proteomes.get_taxon_id("UP999999999").is_none());
}

#[test]
fn sample_store_reads_the_document() {
    let (_dir, store) = load();

    // The sample store exposes no getters, so the only way to observe what it parsed is to
    // serialise it back out.
    let json = serde_json::to_value(store.sample_store()).expect("the sample store should serialise");
    let datasets = &json["sample_data"][0]["datasets"];

    assert_eq!(json["sample_data"][0]["environment"], "Fixture environment");
    assert_eq!(datasets[0]["name"], "Crocodylus peptides");
    assert_eq!(datasets[0]["data"][0], fixtures::peptides::UNIQUE);
}
