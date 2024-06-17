use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_equate_il, default_extra}, AppState};

use super::Query;

#[derive(Deserialize)]
pub struct QueryParams {
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool
}

#[derive(Serialize)]
#[serde(untagged)]
enum EcNumber {
    Default(EcNumberDefault),
    Extra(EcNumberExtra)
}

#[derive(Serialize)]
struct EcNumberDefault {
    ec_number: String,
    protein_count: u32
}

#[derive(Serialize)]
struct EcNumberExtra {
    ec_number: String,
    protein_count: u32,
    name: Option<String>
}

#[derive(Serialize)]
pub struct EcInformation {
    peptide: String,
    total_protein_count: usize,
    ec: Vec<EcNumber>
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra }): Query<QueryParams>
) -> Json<Vec<EcInformation>> {
    let result = index.analyse(&input, equate_il).result;

    Json(result.into_iter().filter_map(|item| {
        if let Some(fa) = item.fa {
            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);

            let ecs = fa.data.iter().filter(|(key, _)| key.starts_with("EC:"));
            let ec: Vec<EcNumber> = if extra {
                let ec_store = datastore.ec_store();
                ecs.map(|(key, count)| EcNumber::Extra(EcNumberExtra {
                    ec_number: key[3..].to_string(),
                    protein_count: *count,
                    name: ec_store.get(&key[3..]).cloned()
                })).collect()
            } else {
                ecs.map(|(key, count)| EcNumber::Default(EcNumberDefault {
                    ec_number: key[3..].to_string(),
                    protein_count: *count
                })).collect()
            };

            Some(EcInformation {
                peptide: item.sequence,
                total_protein_count,
                ec
            })
        } else {
            None
        }
    }).collect())
}
