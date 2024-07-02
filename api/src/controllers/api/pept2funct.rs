use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_domains, default_equate_il, default_extra}, helpers::{ec_helper::{ec_numbers_from_map, EcNumber}, go_helper::{go_terms_from_map, GoTerms}, interpro_helper::{interpro_entries_from_map, InterproEntries}}, AppState};

use super::generate_handlers;

#[derive(Deserialize)]
pub struct Parameters {
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

generate_handlers!(
    async fn handler(
        State(AppState { index, datastore, .. }): State<AppState>,
        Parameters { input, equate_il, extra, domains } => Parameters
    ) -> Json<Vec<FunctInformation>> {
        let result = index.analyse(&input, equate_il).result;
    
        let ec_store = datastore.ec_store();
        let go_store = datastore.go_store();
        let interpro_store = datastore.interpro_store();
    
        Json(result.into_iter().filter_map(|item| {
            let fa = item.fa?;
            
            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);
            let gos = go_terms_from_map(&fa.data, go_store, extra, domains);
            let iprs = interpro_entries_from_map(&fa.data, interpro_store, extra, domains);
    
            Some(FunctInformation {
                peptide: item.sequence,
                total_protein_count,
                ec: ecs,
                go: gos,
                ipr: iprs
            })
        }).collect())
    }
);
