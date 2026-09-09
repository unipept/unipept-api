use std::collections::HashMap;

use datastore::GoStore;
use serde::Serialize;

use crate::helpers::{by_count_then_key, grouped_by_domain, is_zero};

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

/// A term the store knows no namespace for is dropped, which is what `filter_map` says here.
fn handle_domains(gos: Vec<(&str, u32)>, go_store: &GoStore, extra: bool) -> GoTerms {
    GoTerms::Domains(grouped_by_domain(gos.into_iter().filter_map(|(key, count)| {
        let domain = go_store.get_domain(key)?;
        Some((domain.to_string(), go_term(key, count, go_store, extra)))
    })))
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
