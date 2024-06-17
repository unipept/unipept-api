use axum::{extract::State, Json};
use datastore::Lineage;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct Body {
    taxids: Vec<usize>
}

#[derive(Serialize)]
pub struct Taxon {
    id: usize,
    name: String,
    rank: String,
    lineage: Lineage
}

pub async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    body: Json<Body>
) -> Json<Vec<Taxon>> {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Json(body.taxids
        .iter()
        .filter_map(|taxon_id| {
            let (name, rank) = taxon_store.get(*taxon_id as u32)?;
            let lineage = lineage_store.get(*taxon_id as u32)?;
            
            Some(Taxon {
                id: *taxon_id,
                name: name.clone(),
                rank: rank.clone().into(),
                lineage: lineage.clone()
            })
        })
        .collect()
    )
}
