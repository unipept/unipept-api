use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_domains, default_equate_il, default_extra},
        generate_handlers
    },
    helpers::{
        fa_helper::calculate_fa,
        interpro_helper::{interpro_entries_from_map, InterproEntries}
    },
    AppState
};
use crate::helpers::sanitize_peptides;

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
pub struct InterproInformation {
    peptide: String,
    total_protein_count: usize,
    ipr: InterproEntries
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra, domains }: Parameters
) -> Result<Vec<InterproInformation>, ()> {
    let input = sanitize_peptides(input);
    let result = index.analyse(&input, equate_il, false, None);

    let interpro_store = datastore.interpro_store();

    Ok(result
        .into_iter()
        .map(|item| {
            let fa = calculate_fa(&item.proteins);

            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let iprs = interpro_entries_from_map(&fa.data, interpro_store, extra, domains);

            InterproInformation { peptide: item.sequence, total_protein_count, ipr: iprs }
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<InterproInformation>>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
