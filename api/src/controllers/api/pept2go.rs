use std::collections::HashMap;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_cutoff, default_domains, default_equate_il, default_extra},
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{
        distinct_peptides,
        fa_helper::calculate_fa,
        go_helper::{GoTerms, go_terms_from_map},
        laid_over_input, sanitize_peptides
    }
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: Flag,
    #[serde(default = "default_extra")]
    extra: Flag,
    #[serde(default = "default_domains")]
    domains: Flag,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize, Clone)]
pub struct GoInformation {
    peptide: String,
    cutoff_used: bool,
    total_protein_count: usize,
    go: GoTerms
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters {
        input,
        equate_il: Flag(equate_il),
        extra: Flag(extra),
        domains: Flag(domains),
        cutoff
    }: Parameters
) -> Result<Vec<GoInformation>, ApiError> {
    let input = sanitize_peptides(input);
    let distinct = distinct_peptides(&input);

    let result = tokio::task::block_in_place(|| index.analyse(&distinct, equate_il, false, Some(cutoff)));

    let go_store = datastore.go_store();

    // One answer per distinct peptide. Aggregating the annotations and naming the terms out of the
    // datastore depend on the peptide alone, so a peptide named twice pays for them once.
    let rows: HashMap<&str, Vec<GoInformation>> = result
        .iter()
        .map(|item| {
            let fa = calculate_fa(&item.proteins);

            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let gos = go_terms_from_map(&fa.data, go_store, extra, domains);

            (item.sequence, vec![GoInformation {
                peptide: item.sequence.to_string(),
                cutoff_used: item.cutoff_used,
                total_protein_count,
                go: gos
            }])
        })
        .collect();

    Ok(laid_over_input(&input, &rows))
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<GoInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
