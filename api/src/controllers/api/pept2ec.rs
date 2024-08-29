use std::collections::HashMap;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_equate_il, default_extra},
        generate_handlers
    },
    helpers::{
        ec_helper::{ec_numbers_from_map, EcNumber},
        fa_helper::calculate_fa
    },
    AppState
};
use crate::helpers::fa_helper::calculate_ec;
use crate::helpers::sanitize_peptides;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool
}

#[derive(Serialize)]
pub struct EcInformation {
    peptide: String,
    total_protein_count: usize,
    ec: Vec<EcNumber>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra }: Parameters
) -> Result<Vec<EcInformation>, ()> {
    let input = sanitize_peptides(input);

    let mut peptide_counts: HashMap<String, usize> = HashMap::new();
    for peptide in input.into_iter() {
        *peptide_counts.entry(peptide).or_insert(0) += 1;
    }

    let unique_peptides: Vec<String> = peptide_counts.keys().cloned().collect();
    let result = index.analyse(&unique_peptides, equate_il, None);

    let ec_store = datastore.ec_store();

    // Step 6: Duplicate the results according to the original input
    let mut final_results = Vec::new();
    for (unique_peptide, item) in unique_peptides.iter().zip(result.into_iter()) {
        if let Some(count) = peptide_counts.get(unique_peptide) {
            let fa = calculate_ec(item.proteins(&index.searcher));
            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);

            for _ in 0..*count {
                let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);

                final_results.push(EcInformation {
                    peptide: item.sequence.clone(),
                    total_protein_count,
                    ec: ecs,
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
    ) -> Result<Json<Vec<EcInformation>>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
