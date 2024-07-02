use axum::{extract::State, Json};
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::{helpers::lineage_helper::{get_lineage_array, LineageVersion}, AppState};

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
    State(AppState { index, datastore, .. }): State<AppState>,
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
            let lineage = get_lineage_array(item.lca.unwrap() as u32, LineageVersion::V2, lineage_store);
            
            DataItem { 
                sequence: item.sequence, 
                lca: item.lca, 
                lineage,
                fa: item.fa
            }
        }).collect()
    })
}
