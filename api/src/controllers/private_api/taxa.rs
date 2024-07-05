use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::generate_json_handlers, helpers::lineage_helper::{get_lineage_array, LineageVersion}, AppState};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    taxids: Vec<usize>
}

#[derive(Serialize)]
pub struct Taxon {
    id: usize,
    name: String,
    rank: String,
    lineage: Vec<Option<i32>>
}

generate_json_handlers!(
    async fn handler(
        State(AppState { datastore, .. }): State<AppState>,
        Parameters { taxids } => Parameters
    ) -> Vec<Taxon> {
        if taxids.is_empty() {
            return Vec::new();
        }
    
        let taxon_store = datastore.taxon_store();
        let lineage_store = datastore.lineage_store();
    
        taxids
            .into_iter()
            .filter_map(|taxon_id| {
                let (name, rank) = taxon_store.get(taxon_id as u32)?;
                let lineage = get_lineage_array(taxon_id as u32, LineageVersion::V2, lineage_store);
                
                Some(Taxon {
                    id: taxon_id,
                    name: name.clone(),
                    rank: rank.clone().into(),
                    lineage
                })
            })
            .collect()
    }
);
