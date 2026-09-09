use std::collections::HashMap;

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

/// Groups terms under the domain each belongs to, as one single-key map per domain. The domains
/// come out by name, and each keeps the order its terms were given in.
pub fn grouped_by_domain<T>(terms: impl Iterator<Item = (String, T)>) -> Vec<HashMap<String, Vec<T>>> {
    let mut domains: HashMap<String, Vec<T>> = HashMap::new();
    for (domain, term) in terms {
        domains.entry(domain).or_default().push(term);
    }

    let mut domains: Vec<(String, Vec<T>)> = domains.into_iter().collect();
    domains.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

    domains.into_iter().map(|(domain, terms)| HashMap::from([(domain, terms)])).collect()
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
