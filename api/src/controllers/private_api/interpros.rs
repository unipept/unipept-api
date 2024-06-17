use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct Body {
    interpros: Vec<String>
}

#[derive(Serialize)]
pub struct InterproEntry {
    code: String,
    name: String,
    category: String
}

pub async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    body: Json<Body>
) -> Json<Vec<InterproEntry>> {
    Json(body.interpros
        .iter()
        .map(|interpro_entry| interpro_entry.trim())
        .filter_map(|interpro_entry| {
            datastore.interpro_store().get(interpro_entry).map(|(cat, ipr)| InterproEntry {
                code: interpro_entry.to_string(),
                name: ipr.clone(),
                category: cat.clone()
            })
        })
        .collect()
    )
}
