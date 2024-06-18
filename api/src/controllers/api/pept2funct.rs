use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_domains, default_equate_il, default_extra}, helpers::{ec_helper::{ec_numbers, EcNumber}, go_helper::{go_terms, GoTerms}, interpro_helper::{interpro_entries, InterproEntries}}, AppState};

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
pub struct FunctInformation {
    peptide: String,
    total_protein_count: usize,
    ec: Vec<EcNumber>,
    go: GoTerms,
    ipr: InterproEntries
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra, domains }): Query<QueryParams>
) -> Json<Vec<FunctInformation>> {
    let result = index.analyse(&input, equate_il).result;

    let ec_store = datastore.ec_store();
    let go_store = datastore.go_store();
    let interpro_store = datastore.interpro_store();

    Json(result.into_iter().filter_map(|item| {
        let fa = item.fa?;
        
        let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
        let ecs = ec_numbers(&fa.data, ec_store, extra);
        let gos = go_terms(&fa.data, go_store, extra, domains);
        let iprs = interpro_entries(&fa.data, interpro_store, extra, domains);

        Some(FunctInformation {
            peptide: item.sequence,
            total_protein_count,
            ec: ecs,
            go: gos,
            ipr: iprs
        })
    }).collect())
}
