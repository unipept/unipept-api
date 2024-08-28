use std::collections::{HashMap, HashSet};

use index::ProteinInfo;
use serde::Serialize;

/// A struct that represents the functional annotations once aggregated
#[derive(Debug, Serialize)]
pub struct FunctionalAggregation {
    /// A HashMap representing how many GO, EC and IPR terms were found
    pub counts: HashMap<String, usize>,
    /// A HashMap representing how often a certain functional annotation was found
    pub data: HashMap<String, u32>
}

pub fn calculate_ec(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    let mut proteins_with_ec: HashSet<&str> = HashSet::new();

    let mut data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        for ec_number in protein.ec_numbers.split(';') {
            proteins_with_ec.insert(&protein.uniprot_accession); // TODO: outside of loop?
            data.entry(ec_number.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_ec.len());

    FunctionalAggregation { counts, data }
}

pub fn calculate_go(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    let mut proteins_with_go: HashSet<&str> = HashSet::new();

    let mut data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        for go_term in protein.go_terms.split(';') {
            proteins_with_go.insert(&protein.uniprot_accession); // TODO: outside of loop?
            data.entry(go_term.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_go.len());

    FunctionalAggregation { counts, data }
}

pub fn calculate_ipr(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    let mut proteins_with_ipr: HashSet<&str> = HashSet::new();

    let mut data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        for interpro_entry in protein.interpro_entries.split(';') {
            proteins_with_ipr.insert(&protein.uniprot_accession);
            data.entry(interpro_entry.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_ipr.len());

    FunctionalAggregation { counts, data }
}

pub fn calculate_fa(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    // Keep track of the proteins that have any annotation
    let mut proteins_with_annotations: HashSet<&str> = HashSet::new();

    let mut proteins_with_ec: HashSet<&str> = HashSet::new();
    let mut proteins_with_go: HashSet<&str> = HashSet::new();
    let mut proteins_with_ipr: HashSet<&str> = HashSet::new();

    let mut data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        for ec_number in protein.ec_numbers.split(';') {
            proteins_with_ec.insert(&protein.uniprot_accession);
            proteins_with_annotations.insert(&protein.uniprot_accession);
            data.entry(ec_number.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }

        for go_term in protein.go_terms.split(';') {
            proteins_with_go.insert(&protein.uniprot_accession);
            proteins_with_annotations.insert(&protein.uniprot_accession);
            data.entry(go_term.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }

        for interpro_entry in protein.interpro_entries.split(';') {
            proteins_with_ipr.insert(&protein.uniprot_accession);
            proteins_with_annotations.insert(&protein.uniprot_accession);
            data.entry(interpro_entry.to_string()).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_annotations.len());
    counts.insert("EC".to_string(), proteins_with_ec.len());
    counts.insert("GO".to_string(), proteins_with_go.len());
    counts.insert("IPR".to_string(), proteins_with_ipr.len());

    data.remove("");

    FunctionalAggregation { counts, data }
}
