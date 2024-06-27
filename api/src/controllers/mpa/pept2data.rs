use axum::{extract::State, Json};
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::{helpers::{lca_helper::calculate_lca, lineage_helper::{get_lineage_array, LineageVersion}}, AppState};

#[derive(Serialize, Deserialize)]
pub struct Body {
    peptides: Vec<String>,
    equate_il: bool
}

#[derive(Serialize)]
pub struct DataItem {
    sequence: String,
    lca: Option<usize>,
    lineage: Vec<Option<i32>>,
    fa: Option<FunctionalAggregation>
}

#[derive(Serialize)]
pub struct Data {
    peptides: Vec<DataItem>
}

pub async fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Json(Body { mut peptides, equate_il }): Json<Body>
) -> Json<Data> {
    if peptides.is_empty() {
        return Json(Data { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();
    let result = index.analyse(&peptides, equate_il).result;

    let lineage_store = datastore.lineage_store();
    
    Json(Data {
        peptides: result.into_iter().map(|item| {
            let lca = calculate_lca(item.taxa.iter().map(|&taxon_id| taxon_id as u32).collect(), LineageVersion::V2, lineage_store);
            let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);
            
            DataItem { 
                sequence: item.sequence, 
                lca: Some(lca as usize), 
                lineage,
                fa: item.fa
            }
        }).collect()
    })
}
