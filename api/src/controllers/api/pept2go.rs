use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_equate_il, default_extra, default_domains}, AppState};

use super::Query;

#[derive(Deserialize)]
pub struct QueryParams {
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_domains")]
    domains: bool
}

#[derive(Serialize)]
pub struct GoTerm {
    go_term: String,
    protein_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GoInformation {
    Default {
        peptide: String,
        total_protein_count: usize,
        go: Vec<GoTerm>
    },
    Domains {
        peptide: String,
        total_protein_count: usize,
        go: Vec<HashMap<String, Vec<GoTerm>>>
    }
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra, domains }): Query<QueryParams>
) -> Json<Vec<GoInformation>> {
    let result = index.analyse(&input, equate_il).result;
    let go_store = datastore.go_store();

    Json(result.into_iter().filter_map(|item| {
        if let Some(fa) = item.fa {
            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);

            if domains {
                let mut go_domains = HashMap::new();
                for (key, count) in fa.data.iter().filter(|(key, _)| key.starts_with("GO:")) {
                    if let Some(domain) = go_store.get_domain(&key) {
                        go_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(GoTerm {
                            go_term: key.to_string(),
                            protein_count: *count,
                            name: if extra { go_store.get_name(&key).map(|s| s.to_string()) } else { None }
                        });
                    }
                }

                let mut test = Vec::new();
                for (key, value) in go_domains.into_iter() {
                    let mut h = HashMap::new();
                    h.insert(key, value);
                    test.push(h);
                }

                return Some(GoInformation::Domains {
                    peptide: item.sequence,
                    total_protein_count,
                    go: test
                });
            } else {
                return Some(GoInformation::Default {
                    peptide: item.sequence,
                    total_protein_count,
                    go: fa.data
                        .iter()
                        .filter(|(key, _)| key.starts_with("GO:"))
                        .map(|(key, count)| {
                            GoTerm {
                                go_term: key.to_string(),
                                protein_count: *count,
                                name: if extra { go_store.get_name(&key).map(|s| s.to_string()) } else { None }
                            }
                        })
                        .collect()
                });
            }
        } else {
            None
        }
    }).collect())
}
