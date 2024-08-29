use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_domains, default_equate_il, default_extra},
        generate_handlers
    },
    helpers::{
        ec_helper::{ec_numbers_from_map, EcNumber},
        fa_helper::calculate_fa,
        go_helper::{go_terms_from_map, GoTerms},
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
pub struct FunctInformation {
    peptide: String,
    total_protein_count: usize,
    ec: Vec<EcNumber>,
    go: GoTerms,
    ipr: InterproEntries
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra, domains }: Parameters
) -> Result<Vec<FunctInformation>, ()> {
    let input = sanitize_peptides(input);
    let result = index.analyse(&input, equate_il, None);

    let ec_store = datastore.ec_store();
    let go_store = datastore.go_store();
    let interpro_store = datastore.interpro_store();

    Ok(result
        .into_iter()
        .map(|item| {
            let fa = calculate_fa(item.proteins(&index.searcher));

            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);
            let gos = go_terms_from_map(&fa.data, go_store, extra, domains);
            let iprs = interpro_entries_from_map(&fa.data, interpro_store, extra, domains);

            FunctInformation {
                peptide: item.sequence,
                total_protein_count,
                ec: ecs,
                go: gos,
                ipr: iprs
            }
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<FunctInformation>>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
