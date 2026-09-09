use std::collections::{BTreeMap, HashMap};

use itertools::Itertools;

pub mod ec_helper;
pub mod fa_helper;
pub mod filters;
pub mod go_helper;
pub mod interpro_helper;
pub mod lca_helper;
pub mod lineage_helper;
pub mod tree_helper;

fn is_zero(num: &u32) -> bool {
    *num == 0
}

/// Orders the terms of a functional response: most frequent first, ties broken on the identifier
/// so the order is total.
pub fn by_count_then_key<'a>(terms: impl Iterator<Item = (&'a str, u32)>) -> Vec<(&'a str, u32)> {
    let mut terms: Vec<(&str, u32)> = terms.collect();
    terms.sort_unstable_by(|(left_key, left_count), (right_key, right_count)| {
        right_count.cmp(left_count).then_with(|| left_key.cmp(right_key))
    });
    terms
}

/// The terms of one annotation family, ordered, out of the aggregated counts.
///
/// The prefix both selects the family and stays on the key: `ec_number` and `interpro_entry` trim
/// it themselves, and a GO term keeps it.
pub fn family_from_map<'a>(fa_data: &'a HashMap<String, u32>, prefix: &str) -> Vec<(&'a str, u32)> {
    by_count_then_key(
        fa_data.iter().filter(|(key, _)| key.starts_with(prefix)).map(|(key, &count)| (key.as_str(), count))
    )
}

/// The same, out of a list of annotations that carries no counts.
///
/// Every term is given a count of zero, which `is_zero` keeps out of the response, so the
/// identifier alone puts them in order.
pub fn family_from_list<'a>(fa_data: &[&'a str], prefix: &str) -> Vec<(&'a str, u32)> {
    by_count_then_key(fa_data.iter().filter(|key| key.starts_with(prefix)).map(|&key| (key, 0)))
}

/// Groups terms under the domain each belongs to, as one single-key map per domain. The domains
/// come out by name, and each keeps the order its terms were given in.
pub fn grouped_by_domain<T>(terms: impl Iterator<Item = (String, T)>) -> Vec<HashMap<String, Vec<T>>> {
    let mut domains: BTreeMap<String, Vec<T>> = BTreeMap::new();
    for (domain, term) in terms {
        domains.entry(domain).or_default().push(term);
    }

    domains.into_iter().map(|(domain, terms)| HashMap::from([(domain, terms)])).collect()
}

/// The distinct peptides of an input, in first-appearance order.
///
/// The search answers in the order it is given, and `mpa/pept2data` reports that order as its own,
/// so this order reaches a caller.
pub fn distinct_peptides(input: &[String]) -> Vec<String> {
    input.iter().unique().cloned().collect()
}

/// Lays the rows built for each distinct peptide back over the input that asked for them.
///
/// A peptide the index matched nothing for has no rows, and takes no position rather than an empty
/// one.
///
/// The rows are taken by value so that the last position a peptide occupies moves them rather than
/// copying them. An input without repeats therefore copies nothing at all, which is most of them.
pub fn laid_over_input<T: Clone>(input: &[String], mut rows: HashMap<&str, Vec<T>>) -> Vec<T> {
    let mut remaining: HashMap<&str, usize> = HashMap::with_capacity(rows.len());
    for peptide in input {
        *remaining.entry(peptide.as_str()).or_default() += 1;
    }

    let total = remaining.iter().map(|(peptide, count)| rows.get(peptide).map_or(0, Vec::len) * count).sum();
    let mut laid_out = Vec::with_capacity(total);

    for peptide in input {
        let peptide = peptide.as_str();

        let Some(left) = remaining.get_mut(peptide) else { continue };
        *left -= 1;

        if *left == 0 {
            laid_out.extend(rows.remove(peptide).unwrap_or_default());
        } else if let Some(rows) = rows.get(peptide) {
            laid_out.extend_from_slice(rows);
        }
    }

    laid_out
}

pub fn sanitize_peptides(peptides: Vec<String>) -> Vec<String> {
    peptides.into_iter().map(|s| s.trim_end().to_uppercase()).collect()
}

pub fn sanitize_proteins(proteins: Vec<String>) -> Vec<String> {
    proteins.into_iter().map(|s| s.trim_end().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_peptide_named_twice_is_searched_once() {
        let input = ["AAA".to_string(), "BBB".to_string(), "AAA".to_string()];

        assert_eq!(distinct_peptides(&input), vec!["AAA".to_string(), "BBB".to_string()]);
    }

    #[test]
    fn the_rows_of_one_peptide_land_at_each_of_its_positions() {
        let input = ["AAA".to_string(), "BBB".to_string(), "AAA".to_string()];
        let rows = HashMap::from([("AAA", vec!["a"]), ("BBB", vec!["b"])]);

        assert_eq!(laid_over_input(&input, rows), vec!["a", "b", "a"]);
    }

    /// A peptide the index matched nothing for is not in the map, and takes no position rather than
    /// shifting the peptides after it.
    #[test]
    fn a_peptide_with_no_rows_takes_no_position() {
        let input = ["AAA".to_string(), "MISSING".to_string(), "BBB".to_string()];
        let rows = HashMap::from([("AAA", vec!["a"]), ("BBB", vec!["b"])]);

        assert_eq!(laid_over_input(&input, rows), vec!["a", "b"]);
    }

    /// A peptide occupying several positions carries its rows to each, so the rows have to survive
    /// being moved out at the last one.
    #[test]
    fn a_peptide_at_several_positions_carries_its_rows_to_all_of_them() {
        let input = ["AAA".to_string(), "BBB".to_string(), "AAA".to_string(), "AAA".to_string()];
        let rows = HashMap::from([("AAA", vec!["a"]), ("BBB", vec!["b"])]);

        assert_eq!(laid_over_input(&input, rows), vec!["a", "b", "a", "a"]);
    }

    /// `pept2taxa` and `pept2prot` answer with more than one row per peptide, and each repeat
    /// carries all of them.
    #[test]
    fn a_peptide_answering_with_several_rows_repeats_all_of_them() {
        let input = ["AAA".to_string(), "AAA".to_string()];
        let rows = HashMap::from([("AAA", vec!["a1", "a2"])]);

        assert_eq!(laid_over_input(&input, rows), vec!["a1", "a2", "a1", "a2"]);
    }

    #[test]
    fn the_most_frequent_term_comes_first() {
        let ordered = by_count_then_key([("GO:0002", 1), ("GO:0001", 5)].into_iter());

        assert_eq!(ordered, vec![("GO:0001", 5), ("GO:0002", 1)]);
    }

    /// Two terms of one count are separated by the identifier alone.
    #[test]
    fn terms_of_one_count_go_out_by_identifier() {
        let ordered = by_count_then_key([("GO:0009279", 2), ("GO:0005515", 2)].into_iter());

        assert_eq!(ordered, vec![("GO:0005515", 2), ("GO:0009279", 2)]);
    }

    /// `*_from_list` has no counts to rank by and passes zero for every term, which leaves the
    /// identifier to order them.
    #[test]
    fn terms_without_counts_keep_the_identifier_order() {
        let ordered = by_count_then_key([("EC:2.7.11.1", 0), ("EC:1.1.1.1", 0)].into_iter());

        assert_eq!(ordered, vec![("EC:1.1.1.1", 0), ("EC:2.7.11.1", 0)]);
    }
}
