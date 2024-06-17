use std::collections::HashSet;

use axum::{extract::State, Json};
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct Body {
    peptides: Vec<String>,
    taxa: Vec<usize>,
    equate_il: Option<bool>
}

#[derive(Serialize)]
pub struct FilteredData {
    sequence: String,
    taxa: Vec<usize>,
    fa: Option<FunctionalAggregation>
}

pub async fn handler(
    State(AppState { index, .. }): State<AppState>,
    body: Json<Body>
) -> Json<Vec<FilteredData>> {
    let result = index.analyse(&body.peptides, body.equate_il.unwrap_or_default()).result;
    let taxa_set: HashSet<usize> = HashSet::from_iter(body.taxa.iter().cloned());
    Json(result.into_iter().map(|item| {   
        FilteredData { 
            sequence: item.sequence, 
            taxa: HashSet::from_iter(item.taxa.iter().cloned()).intersection(&taxa_set).cloned().collect(),
            fa: item.fa
        }
    }).collect())
}
