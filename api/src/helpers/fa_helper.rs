use std::collections::{HashMap, HashSet};

use index::{ProteinInfo, fa_compression::algorithm1::decode};
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
    let mut proteins_with_annotations: HashSet<&str> = HashSet::new();

    // Keep track of the proteins that have a certain annotation
    let mut proteins_with_ec: HashSet<&str> = HashSet::new();
    let mut proteins_with_go: HashSet<&str> = HashSet::new();
    let mut proteins_with_ipr: HashSet<&str> = HashSet::new();

    // Keep track of the counts of the different annotations
    let mut data: HashMap<String, u32> = HashMap::new();

    for protein in proteins.iter() {
        // The index hands back the annotations still encoded, so this is where they are decoded —
        // after the caller's filters have run, rather than for every hit the index examined.
        let annotations = decode(protein.annotations);

        for annotation in annotations.split(';') {
            match annotation.chars().next() {
                Some('E') => {
                    proteins_with_ec.insert(protein.uniprot_accession);
                    proteins_with_annotations.insert(protein.uniprot_accession);
                }
                Some('G') => {
                    proteins_with_go.insert(protein.uniprot_accession);
                    proteins_with_annotations.insert(protein.uniprot_accession);
                }
                Some('I') => {
                    proteins_with_ipr.insert(protein.uniprot_accession);
                    proteins_with_annotations.insert(protein.uniprot_accession);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(annotations: &str) -> Vec<u8> {
        index::fa_compression::algorithm1::encode(annotations)
    }

    fn protein<'a>(accession: &'a str, annotations: &'a [u8]) -> ProteinInfo<'a> {
        ProteinInfo { taxon: 1, uniprot_accession: accession, annotations }
    }

    #[test]
    fn counts_proteins_while_data_counts_occurrences() {
        let (a, b, c) = (encoded("GO:0001;EC:1.1.1.1"), encoded("GO:0001;IPR:IPR001"), encoded(""));
        let proteins = [protein("P1", &a), protein("P2", &b), protein("P3", &c)];

        let fa = calculate_fa(&proteins);

        assert_eq!(fa.data.get("GO:0001"), Some(&2));
        assert_eq!(fa.data.get("EC:1.1.1.1"), Some(&1));
        assert_eq!(fa.data.get("IPR:IPR001"), Some(&1));

        assert_eq!(fa.counts.get("all"), Some(&2));
        assert_eq!(fa.counts.get("EC"), Some(&1));
        assert_eq!(fa.counts.get("GO"), Some(&2));
        assert_eq!(fa.counts.get("IPR"), Some(&1));
    }

    /// A protein carrying no annotations must not leave an empty key behind.
    #[test]
    fn the_empty_annotation_is_not_reported() {
        let none = encoded("");
        let fa = calculate_fa(&[protein("P1", &none)]);

        assert!(!fa.data.contains_key(""));
        assert_eq!(fa.counts.get("all"), Some(&0));
    }

    /// `counts` is per protein and `data` is per occurrence, so two entries for one accession
    /// count once in the first and twice in the second.
    #[test]
    fn one_accession_twice_counts_as_one_protein() {
        let go = encoded("GO:0001");
        let fa = calculate_fa(&[protein("P1", &go), protein("P1", &go)]);

        assert_eq!(fa.counts.get("all"), Some(&1));
        assert_eq!(fa.counts.get("GO"), Some(&1));
        assert_eq!(fa.data.get("GO:0001"), Some(&2));
    }

    #[test]
    fn no_proteins_reports_zero_rather_than_nothing() {
        let fa = calculate_fa(&[]);

        assert_eq!(fa.counts.get("all"), Some(&0));
        assert_eq!(fa.counts.get("EC"), Some(&0));
        assert_eq!(fa.counts.get("GO"), Some(&0));
        assert_eq!(fa.counts.get("IPR"), Some(&0));
        assert!(fa.data.is_empty());
    }
}
