use std::collections::HashMap;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_cutoff, default_equate_il, default_extra},
        generate_handlers
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
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize)]
pub struct EcInformation {
    peptide: String,
    cutoff_used: bool,
    total_protein_count: usize,
    ec: Vec<EcNumber>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra, cutoff }: Parameters
) -> Result<Vec<EcInformation>, ApiError> {
    let input = sanitize_peptides(input);

    let mut peptide_counts: HashMap<String, usize> = HashMap::new();
    for peptide in input.into_iter() {
        *peptide_counts.entry(peptide).or_insert(0) += 1;
    }

    let unique_peptides: Vec<String> = peptide_counts.keys().cloned().collect();
    let result = tokio::task::block_in_place(|| index.analyse(&unique_peptides, equate_il, false, Some(cutoff)));

    let ec_store = datastore.ec_store();

    // Step 6: Duplicate the results according to the original input
    let mut final_results = Vec::new();
    for (unique_peptide, item) in unique_peptides.iter().zip(result) {
        if let Some(count) = peptide_counts.get(unique_peptide) {
            let fa = calculate_fa(&item.proteins);
            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let cutoff_used = item.cutoff_used;

            for _ in 0..*count {
                let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);

                final_results.push(EcInformation {
                    peptide: item.sequence.to_string(),
                    cutoff_used,
                    total_protein_count,
                    ec: ecs
                });
            }
        }
    }

    Ok(final_results)
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<EcInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
