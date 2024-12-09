use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{generate_handlers, mpa::default_equate_il, mpa::default_tryptic, api::default_cutoff},
    helpers::{
        fa_helper::{calculate_fa, FunctionalAggregation},
        lca_helper::calculate_lca,
        lineage_helper::{get_lineage_array, LineageVersion}
    },
    AppState
};
use crate::helpers::sanitize_peptides;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    peptides: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_tryptic")]
    tryptic: bool,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize)]
pub struct DataItem {
    sequence: String,
    lca: Option<u32>,
    lineage: Vec<Option<i32>>,
    fa: FunctionalAggregation
}

#[derive(Serialize)]
pub struct Data {
    peptides: Vec<DataItem>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { mut peptides, equate_il, tryptic, cutoff }: Parameters
) -> Result<Data, ()> {
    if peptides.is_empty() {
        return Ok(Data { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();

    let peptides = sanitize_peptides(peptides);
    let result = index.analyse(&peptides, equate_il, Some(cutoff), Some(tryptic));

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Ok(Data {
        peptides: result
            .into_iter()
            .map(|item| {
                let lca = calculate_lca(
                    item.proteins.iter().map(|protein| protein.taxon).collect(),
                    LineageVersion::V2,
                    taxon_store,
                    lineage_store
                );
                let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);

                DataItem {
                    sequence: item.sequence,
                    lca: Some(lca as u32),
                    lineage,
                    fa: calculate_fa(&item.proteins)
                }
            })
            .collect()
    })
}

generate_handlers!(
    async fn json_handler(
        state=> State<AppState>,
        params => Parameters
    ) -> Result<Json<Data>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
