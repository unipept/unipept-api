use std::collections::HashMap;

use datastore::GoStore;
use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub enum GoTerm {
    Default {
        go_term: String,
        protein_count: u32,
    },
    Extra {
        go_term: String,
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

pub fn go_terms(fa_data: &HashMap<String, u32>, go_store: &GoStore, extra: bool, domains: bool) -> GoTerms {
    let gos = fa_data.into_iter().filter(|(key, _)| key.starts_with("GO:"));

    if domains {
        let mut go_domains = HashMap::new();
        for (key, &count) in gos {
            if let Some(domain) = go_store.get_domain(&key) {
                if extra {
                    go_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(GoTerm::Extra {
                        go_term: key.to_string(),
                        protein_count: count,
                        name: go_store.get_name(&key).map(|s| s.to_string()).unwrap_or_default()
                    });
                } else {
                    go_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(GoTerm::Default {
                        go_term: key.to_string(),
                        protein_count: count
                    });
                }
            }
        }

        let mut result = Vec::new();
        for (key, value) in go_domains.into_iter() {
            let mut mapping = HashMap::new();
            mapping.insert(key, value);
            result.push(mapping);
        }

        GoTerms::Domains(result)
    } else if extra {
        GoTerms::Default(
            gos.map(|(key, &count)| GoTerm::Extra {
                go_term: key.to_string(),
                protein_count: count,
                name: go_store.get_name(&key).map(|s| s.to_string()).unwrap_or_default()
            }).collect()
        )
    } else {
        GoTerms::Default(
            gos.map(|(key, &count)| GoTerm::Default {
                go_term: key.to_string(),
                protein_count: count
            }).collect()
        )
    }
}
