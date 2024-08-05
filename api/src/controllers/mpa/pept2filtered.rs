use std::collections::HashSet;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{generate_handlers, mpa::default_equate_il}, helpers::fa_helper::{calculate_fa, FunctionalAggregation}, AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    peptides: Vec<String>,
    #[serde(default)]
    taxa: Vec<u32>,
    #[serde(default = "default_equate_il")]
    equate_il: bool
}

#[derive(Serialize)]
pub struct FilteredDataItem {
    sequence: String,
    taxa: Vec<u32>,
    fa: FunctionalAggregation
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
    let result = index.analyse(&peptides, equate_il);

    let taxa_set: HashSet<u32> = HashSet::from_iter(taxa.iter().cloned());

    Ok(FilteredData {
        peptides: result
            .into_iter()
            .filter_map(|item| {
                let item_taxa = item.proteins.iter().map(|protein| protein.taxon).collect::<HashSet<u32>>();
                let mut intersection = item_taxa.intersection(&taxa_set);

                // Skip if no intersection
                intersection.next()?;

                Some(FilteredDataItem { sequence: item.sequence, taxa: intersection.cloned().collect(), fa: calculate_fa(&item.proteins) })
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
