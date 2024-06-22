use std::collections::HashSet;

use axum::{extract::State, Json};
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::{controllers::mpa::default_equate_il, AppState};

#[derive(Serialize, Deserialize)]
pub struct Body {
    peptides: Vec<String>,
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

pub async fn handler(
    State(AppState { index, .. }): State<AppState>,
    Json(Body { mut peptides, taxa, equate_il }): Json<Body>
) -> Json<FilteredData> {
    if peptides.is_empty() {
        return Json(FilteredData { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();
    let result = index.analyse(&peptides, equate_il).result;

    let taxa_set: HashSet<usize> = HashSet::from_iter(taxa.iter().cloned());

    Json(FilteredData {
        peptides: result.into_iter().filter_map(|mut item| {
            item.taxa = HashSet::from_iter(item.taxa.iter().cloned()).intersection(&taxa_set).cloned().collect();
            
            if item.taxa.is_empty() {
                return None;
            }
            
            Some(FilteredDataItem { 
                sequence: item.sequence, 
                taxa: item.taxa,
                fa: item.fa
            })
        }).collect()
    })
}
