use std::collections::HashMap;

use datastore::GoStore;
use serde::Serialize;

use crate::helpers::is_zero;

#[derive(Serialize)]
#[serde(untagged)]
pub enum GoTerm {
    Default {
        go_term: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
    },
    Extra {
        go_term: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
        name: String,
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GoTerms {
    Default (Vec<GoTerm>),
    Domains (Vec<HashMap<String, Vec<GoTerm>>>)
}

pub fn go_terms_from_map(fa_data: &HashMap<String, u32>, go_store: &GoStore, extra: bool, domains: bool) -> GoTerms {
    let go_terms = fa_data
        .into_iter()
        .filter(|(key, _)| key.starts_with("GO:"));

    if domains {
        handle_domains(go_terms.map(|(key, count)| (key.as_str(), count)), go_store, extra)
    } else {
        GoTerms::Default(
            go_terms
                .map(|(key, &count)| go_term(key, count, go_store, extra))
                .collect()
        )
    }
}

pub fn go_terms_from_list(fa_data: &Vec<&str>, go_store: &GoStore, extra: bool, domains: bool) -> GoTerms {
    let go_terms = fa_data
        .iter()
        .filter(|key| key.starts_with("GO:"));

    if domains {
        handle_domains(go_terms.map(|&key| (key, &0u32)), go_store, extra)
    } else {
        GoTerms::Default(
            go_terms
                .map(|key| go_term(key, 0, go_store, extra))
                .collect()
        )
    }
}

fn handle_domains<'a>(gos: impl Iterator<Item = (&'a str, &'a u32)>, go_store: &GoStore, extra: bool) -> GoTerms {
    let mut go_domains = HashMap::new();
    for (key, &count) in gos {
        if let Some(domain) = go_store.get_domain(key) {
            go_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(go_term(key, count, go_store, extra));
        }
    }

    let result: Vec<HashMap<String, Vec<GoTerm>>> = go_domains.into_iter()
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
            name: go_store.get_name(key).map(|s| s.to_string()).unwrap_or_default(),
        }
    } else {
        GoTerm::Default {
            go_term: key.to_string(),
            protein_count: count,
        }
    }
}
