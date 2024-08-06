use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_domains, default_equate_il, default_extra},
        generate_handlers
    },
    helpers::{
        fa_helper::calculate_fa,
        go_helper::{go_terms_from_map, GoTerms}
    },
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
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

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra, domains }: Parameters
) -> Result<Vec<GoInformation>, ()> {
    let result = index.analyse(&input, equate_il);

    let go_store = datastore.go_store();

    Ok(result
        .into_iter()
        .map(|item| {
            let fa = calculate_fa(&item.proteins);

            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let gos = go_terms_from_map(&fa.data, go_store, extra, domains);

            GoInformation { peptide: item.sequence, total_protein_count, go: gos }
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<GoInformation>>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
