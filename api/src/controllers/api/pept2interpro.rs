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
pub struct InterproEntry {
    code: String,
    protein_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum InterproInformation {
    Default {
        peptide: String,
        total_protein_count: usize,
        ipr: Vec<InterproEntry>
    },
    Domains {
        peptide: String,
        total_protein_count: usize,
        ipr: Vec<HashMap<String, Vec<InterproEntry>>>
    }
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra, domains }): Query<QueryParams>
) -> Json<Vec<InterproInformation>> {
    let result = index.analyse(&input, equate_il).result;
    let interpro_store = datastore.interpro_store();

    Json(result.into_iter().filter_map(|item| {
        if let Some(fa) = item.fa {
            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);

            if domains {
                let mut interpro_domains = HashMap::new();
                for (key, count) in fa.data.iter().filter(|(key, _)| key.starts_with("IPR:")) {
                    if let Some(domain) = interpro_store.get_domain(&key[4..]) {
                        interpro_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(InterproEntry {
                            code: key[4..].to_string(),
                            protein_count: *count,
                            name: if extra { interpro_store.get_name(&key[4..]).map(|s| s.to_string()) } else { None }
                        });
                    }
                }

                let mut test = Vec::new();
                for (key, value) in interpro_domains.into_iter() {
                    let mut h = HashMap::new();
                    h.insert(key, value);
                    test.push(h);
                }

                return Some(InterproInformation::Domains {
                    peptide: item.sequence,
                    total_protein_count,
                    ipr: test
                });
            } else {
                return Some(InterproInformation::Default {
                    peptide: item.sequence,
                    total_protein_count,
                    ipr: fa.data
                        .iter()
                        .filter(|(key, _)| key.starts_with("IPR:"))
                        .map(|(key, count)| {
                            InterproEntry {
                                code: key[4..].to_string(),
                                protein_count: *count,
                                name: if extra { interpro_store.get_name(&key[4..]).map(|s| s.to_string()) } else { None }
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
