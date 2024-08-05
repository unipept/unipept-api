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

pub fn calculate_fa(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    // Keep track of the proteins that have any annotation
    let mut proteins_with_annotations: HashSet<String> = HashSet::new();

    // Keep track of the proteins that have a certain annotation
    let mut proteins_with_ec: HashSet<String> = HashSet::new();
    let mut proteins_with_go: HashSet<String> = HashSet::new();
    let mut proteins_with_ipr: HashSet<String> = HashSet::new();

    // Keep track of the counts of the different annotations
    let mut data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        for annotation in protein.functional_annotations.split(';') {
            match annotation.chars().next() {
                Some('E') => {
                    proteins_with_ec.insert(protein.uniprot_accession.clone());
                    proteins_with_annotations.insert(protein.uniprot_accession.clone());
                }
                Some('G') => {
                    proteins_with_go.insert(protein.uniprot_accession.clone());
                    proteins_with_annotations.insert(protein.uniprot_accession.clone());
                }
                Some('I') => {
                    proteins_with_ipr.insert(protein.uniprot_accession.clone());
                    proteins_with_annotations.insert(protein.uniprot_accession.clone());
                }
                _ => {}
            };

            data.entry(annotation.to_string()).and_modify(|c| *c += 1).or_insert(1);
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
