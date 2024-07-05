use axum::{
    extract::State,
    Json
};
use serde::{
    Deserialize,
    Serialize
};

use crate::{
    controllers::{
        api::{
            default_equate_il,
            default_extra
        },
        generate_json_handlers
    },
    helpers::ec_helper::{
        ec_numbers_from_map,
        EcNumber
    },
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    input:     Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra:     bool
}

#[derive(Serialize)]
pub struct EcInformation {
    peptide:             String,
    total_protein_count: usize,
    ec:                  Vec<EcNumber>
}

generate_json_handlers!(
    async fn handler(
        State(AppState { index, datastore, .. }): State<AppState>,
        Parameters { input, equate_il, extra } => Parameters
    ) -> Vec<EcInformation> {
        let result = index.analyse(&input, equate_il).result;

        let ec_store = datastore.ec_store();

        result.into_iter().filter_map(|item| {
            let fa = item.fa?;

            let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);

            Some(EcInformation {
                peptide: item.sequence,
                total_protein_count,
                ec: ecs
            })
        }).collect()
    }
);
