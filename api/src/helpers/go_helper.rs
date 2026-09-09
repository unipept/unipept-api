use std::collections::HashMap;

use datastore::GoStore;
use serde::Serialize;

use crate::helpers::{by_count_then_key, is_zero};

#[derive(Serialize)]
#[serde(untagged)]
pub enum GoTerm {
    Default {
        go_term: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32
    },
    Extra {
        go_term: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
        name: String
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GoTerms {
    Default(Vec<GoTerm>),
    Domains(Vec<HashMap<String, Vec<GoTerm>>>)
}

pub fn go_terms_from_map(fa_data: &HashMap<String, u32>, go_store: &GoStore, extra: bool, domains: bool) -> GoTerms {
    let go_terms = by_count_then_key(
        fa_data.iter().filter(|(key, _)| key.starts_with("GO:")).map(|(key, &count)| (key.as_str(), count))
    );

    if domains {
        handle_domains(go_terms, go_store, extra)
    } else {
        GoTerms::Default(go_terms.into_iter().map(|(key, count)| go_term(key, count, go_store, extra)).collect())
    }
}

pub fn go_terms_from_list(fa_data: &[&str], go_store: &GoStore, extra: bool, domains: bool) -> GoTerms {
    let go_terms = by_count_then_key(fa_data.iter().filter(|key| key.starts_with("GO:")).map(|&key| (key, 0)));

    if domains {
        handle_domains(go_terms, go_store, extra)
    } else {
        GoTerms::Default(go_terms.into_iter().map(|(key, count)| go_term(key, count, go_store, extra)).collect())
    }
}

/// The terms arrive ordered, so each domain keeps them in that order; the domains themselves are
/// grouped through a `HashMap` and so need ordering of their own. There is no count to rank a
/// domain by, so they go out by name.
fn handle_domains(gos: Vec<(&str, u32)>, go_store: &GoStore, extra: bool) -> GoTerms {
    let mut go_domains: HashMap<String, Vec<GoTerm>> = HashMap::new();
    for (key, count) in gos {
        if let Some(domain) = go_store.get_domain(key) {
            go_domains.entry(domain.to_string()).or_default().push(go_term(key, count, go_store, extra));
        }
    }

    let mut go_domains: Vec<(String, Vec<GoTerm>)> = go_domains.into_iter().collect();
    go_domains.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

    let result: Vec<HashMap<String, Vec<GoTerm>>> = go_domains
        .into_iter()
        .map(|(key, value)| {
            let mut mapping = HashMap::new();
            mapping.insert(key, value);
            mapping
        })
        .collect();

    GoTerms::Domains(result)
}

fn go_term(key: &str, count: u32, go_store: &GoStore, extra: bool) -> GoTerm {
    if extra {
        GoTerm::Extra {
            go_term: key.to_string(),
            protein_count: count,
            name: go_store.get_name(key).map(|s| s.to_string()).unwrap_or_default()
        }
    } else {
        GoTerm::Default { go_term: key.to_string(), protein_count: count }
    }
}
