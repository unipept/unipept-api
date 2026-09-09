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

    // Each distinct peptide once. The search reads this list, so the results come back in its
    // order, and a peptide named twice is searched once.
    let distinct: Vec<String> = input.iter().cloned().unique().collect();

    let result = tokio::task::block_in_place(|| index.analyse(&distinct, equate_il, false, Some(cutoff)));

    let ec_store = datastore.ec_store();

    // One answer per distinct peptide. Aggregating the annotations, ordering the terms and naming
    // them out of the datastore all depend on the peptide alone, so a peptide asked for twenty
    // times pays for them once.
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

    // Laid back over the input, so a peptide is answered at each position it was named at. A
    // peptide the index matched nothing for has no answer and is passed over, as it is elsewhere.
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
