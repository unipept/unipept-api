use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_domains, default_equate_il, default_extra}, helpers::go_helper::{go_terms, GoTerms}, AppState};

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
pub struct GoInformation {
    peptide: String,
    total_protein_count: usize,
    go: GoTerms
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra, domains }): Query<QueryParams>
) -> Json<Vec<GoInformation>> {
    let result = index.analyse(&input, equate_il).result;

    let go_store = datastore.go_store();

    Json(result.into_iter().filter_map(|item| {
        let fa = item.fa?;

        let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
        let gos = go_terms(&fa.data, go_store, extra, domains);

        Some(GoInformation {
            peptide: item.sequence,
            total_protein_count,
            go: gos
        })
    }).collect())
}
