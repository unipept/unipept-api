use axum::{extract::State, Json};
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::{controllers::generate_handlers, helpers::{lca_helper::calculate_lca, lineage_helper::{get_lineage_array, LineageVersion}}, AppState};

#[derive(Deserialize)]
pub struct Parameters {
    peptides: Vec<String>,
    equate_il: bool
}

#[derive(Serialize)]
pub struct DataItem {
    sequence: String,
    lca: Option<u32>,
    lineage: Vec<Option<i32>>,
    fa: Option<FunctionalAggregation>
}

#[derive(Serialize)]
pub struct Data {
    peptides: Vec<DataItem>
}

generate_handlers!(
    async fn handler(
        State(AppState { index, datastore, .. }): State<AppState>,
        Parameters { mut peptides, equate_il } => Parameters
    ) -> Data {
        if peptides.is_empty() {
            return Data { peptides: Vec::new() };
        }
    
        peptides.sort();
        peptides.dedup();
        let result = index.analyse(&peptides, equate_il).result;
    
        let lineage_store = datastore.lineage_store();
        
        Data {
            peptides: result.into_iter().map(|item| {
                let lca = calculate_lca(item.taxa.iter().map(|&taxon_id| taxon_id as u32).collect(), LineageVersion::V2, lineage_store);
                let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);
                
                DataItem { 
                    sequence: item.sequence, 
                    lca: Some(lca as u32), 
                    lineage,
                    fa: item.fa
                }
            }).collect()
        }
    }
);
