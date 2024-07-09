use std::collections::HashSet;

use axum::{extract::State, Json};
use index::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{generate_handlers, mpa::default_equate_il},
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    peptides: Vec<String>,
    #[serde(default)]
    taxa: Vec<usize>,
    #[serde(default = "default_equate_il")]
    equate_il: bool
}

#[derive(Serialize)]
pub struct FilteredDataItem {
    sequence: String,
    taxa: Vec<usize>,
    fa: Option<FunctionalAggregation>
}

#[derive(Serialize)]
pub struct FilteredData {
    peptides: Vec<FilteredDataItem>
}

async fn handler(
    State(AppState { index, .. }): State<AppState>,
    Parameters { mut peptides, taxa, equate_il }: Parameters
) -> Result<FilteredData, ()> {
    if peptides.is_empty() {
        return Ok(FilteredData { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();
    let result = index.analyse(&peptides, equate_il).result;

    let taxa_set: HashSet<usize> = HashSet::from_iter(taxa.iter().cloned());

    Ok(FilteredData {
        peptides: result
            .into_iter()
            .filter_map(|mut item| {
                item.taxa = HashSet::from_iter(item.taxa.iter().cloned()).intersection(&taxa_set).cloned().collect();

                if item.taxa.is_empty() {
                    return None;
                }

                Some(FilteredDataItem { sequence: item.sequence, taxa: item.taxa, fa: item.fa })
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
