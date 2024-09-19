use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{generate_handlers, mpa::default_equate_il, mpa::default_include_fa, mpa::default_tryptic},
    helpers::fa_helper::{calculate_fa, FunctionalAggregation},
    AppState
};
use crate::helpers::sanitize_peptides;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    peptides: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_include_fa")]
    include_fa: bool,
    #[serde(default = "default_tryptic")]
    tryptic: bool
}

#[derive(Serialize)]
pub struct FilteredDataItem {
    sequence: String,
    taxa: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fa: Option<FunctionalAggregation>
}

#[derive(Serialize)]
pub struct FilteredData {
    peptides: Vec<FilteredDataItem>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { mut peptides, equate_il, include_fa, tryptic }: Parameters
) -> Result<FilteredData, ()> {
    if peptides.is_empty() {
        return Ok(FilteredData { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();

    let peptides = sanitize_peptides(peptides);
    let result = index.analyse(&peptides, equate_il, Some(10_000), Some(tryptic));

    let taxon_store = datastore.taxon_store();

    Ok(FilteredData {
        peptides: result
            .into_iter()
            .filter_map(|item| {
                let item_taxa: Vec<u32> = item.proteins.iter().map(|protein| protein.taxon).filter(|&taxon_id| taxon_store.is_valid(taxon_id)).collect();

                if item_taxa.is_empty() {
                    return None;
                }

                let fa = if include_fa {
                    Some(calculate_fa(&item.proteins))
                } else {
                    None
                };

                Some(FilteredDataItem {
                    sequence: item.sequence,
                    taxa: item_taxa,
                    fa
                })
            })
            .collect()
    })
}

generate_handlers!(
    async fn json_handler(
        state=> State<AppState>,
        params => Parameters
    ) -> Result<Json<FilteredData>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
