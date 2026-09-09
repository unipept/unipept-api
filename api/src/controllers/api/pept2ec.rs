use std::collections::HashMap;

use axum::{Json, extract::State};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_cutoff, default_equate_il, default_extra},
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{
        ec_helper::{EcNumber, ec_numbers_from_map},
        fa_helper::calculate_fa,
        sanitize_peptides
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
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize, Clone)]
pub struct EcInformation {
    peptide: String,
    cutoff_used: bool,
    total_protein_count: usize,
    ec: Vec<EcNumber>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters {
        input,
        equate_il: Flag(equate_il),
        extra: Flag(extra),
        cutoff
    }: Parameters
) -> Result<Vec<EcInformation>, ApiError> {
    let input = sanitize_peptides(input);
    let distinct: Vec<String> = input.iter().unique().cloned().collect();

    let result = tokio::task::block_in_place(|| index.analyse(&distinct, equate_il, false, Some(cutoff)));

    let ec_store = datastore.ec_store();

    let answers: HashMap<&str, EcInformation> = result
        .iter()
        .map(|item| {
            let fa = calculate_fa(&item.proteins);

            (item.sequence, EcInformation {
                peptide: item.sequence.to_string(),
                cutoff_used: item.cutoff_used,
                total_protein_count: *fa.counts.get("all").unwrap_or(&0),
                ec: ec_numbers_from_map(&fa.data, ec_store, extra)
            })
        })
        .collect();

    // A peptide the index matched nothing for has no answer and takes no position.
    Ok(input.iter().filter_map(|peptide| answers.get(peptide.as_str()).cloned()).collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<EcInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
