use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{helpers::lineage_helper::{get_lineage_array, LineageVersion}, AppState};

#[derive(Serialize, Deserialize)]
pub struct Body {
    taxids: Vec<usize>
}

#[derive(Serialize)]
pub struct Taxon {
    id: usize,
    name: String,
    rank: String,
    lineage: Vec<Option<i32>>
}

pub async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Json(Body { taxids }): Json<Body>
) -> Json<Vec<Taxon>> {
    if taxids.is_empty() {
        return Json(Vec::new());
    }

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Json(taxids
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
    )
}
