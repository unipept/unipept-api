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

fn count_annotations(
    proteins: &[ProteinInfo],
    annotation_prefix: char
) -> (HashSet<String>, HashMap<String, u32>) {
    let mut proteins_with_annotation: HashSet<String> = HashSet::new();
    let mut protein_data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        for annotation in protein.functional_annotations.split(';') {
            match annotation.chars().next() {
                Some(c) => {
                    if c == annotation_prefix {
                        proteins_with_annotation.insert(protein.uniprot_accession.clone());
                        proteins_with_annotation.insert(protein.uniprot_accession.clone());
                        protein_data.entry(annotation.to_string()).and_modify(|c| *c += 1).or_insert(1);
                    }
                }
                _ => {}
            };
        }
    }

    (proteins_with_annotation, protein_data)
}

pub fn calculate_ec(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    let (proteins_with_ec, ec_protein_data) = count_annotations(proteins, 'E');

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_ec.len());

    FunctionalAggregation { counts, data: ec_protein_data }
}

pub fn calculate_go(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    let (proteins_with_go, go_protein_data) = count_annotations(proteins, 'G');

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_go.len());

    FunctionalAggregation { counts, data: go_protein_data }
}

pub fn calculate_ipr(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    let (proteins_with_ipr, ipr_protein_data) = count_annotations(proteins, 'I');

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_ipr.len());

    FunctionalAggregation { counts, data: ipr_protein_data }
}

pub fn calculate_fa(proteins: &[ProteinInfo]) -> FunctionalAggregation {
    // Keep track of the proteins that have a certain annotation
    let (proteins_with_ec, ec_protein_data) = count_annotations(proteins, 'E');
    let (proteins_with_go, go_protein_data) = count_annotations(proteins, 'G');
    let (proteins_with_ipr, ipr_protein_data) = count_annotations(proteins, 'I');

    // Keep track of the counts of the different annotations
    let mut data: HashMap<String, u32> = HashMap::new();

    data.extend(ec_protein_data);
    data.extend(go_protein_data);
    data.extend(ipr_protein_data);

    let mut counts: HashMap<String, usize> = HashMap::new();
    counts.insert("all".to_string(), proteins_with_ec.len() + proteins_with_go.len() + proteins_with_ipr.len());
    counts.insert("EC".to_string(), proteins_with_ec.len());
    counts.insert("GO".to_string(), proteins_with_go.len());
    counts.insert("IPR".to_string(), proteins_with_ipr.len());

    data.remove("");

    FunctionalAggregation { counts, data }
}
