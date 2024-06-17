use axum::{extract::State, Json};
use datastore::Lineage;
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct Body {
    peptides: Vec<String>,
    equate_il: bool,
    missed: bool
}

#[derive(Serialize)]
pub struct Data {
    sequence: String,
    lca: Option<usize>,
    lineage: Lineage,
    fa: Option<FunctionalAggregation>
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    body: Json<Body>
) -> Json<Vec<Data>> {
    let lineage_store = datastore.lineage_store();
    let result = index.analyse(&body.peptides, body.equate_il).result;
    Json(result.into_iter().map(|item| {
        let lineage = lineage_store.get(item.lca.unwrap() as u32).unwrap().clone();
        
        Data { 
            sequence: item.sequence, 
            lca: item.lca, 
            lineage, 
            fa: item.fa
        }
    }).collect())
}
