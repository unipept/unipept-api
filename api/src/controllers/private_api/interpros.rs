use axum::{
    extract::State,
    Json
};
use serde::{
    Deserialize,
    Serialize
};

use crate::{
    controllers::generate_json_handlers,
    AppState
};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    interpros: Vec<String>
}

#[derive(Serialize)]
pub struct InterproEntry {
    code:     String,
    name:     String,
    category: String
}

generate_json_handlers!(
    async fn handler(
        State(AppState { datastore, .. }): State<AppState>,
        Parameters { interpros } => Parameters
    ) -> Result<Vec<InterproEntry>, ()> {
        Ok(interpros
            .iter()
            .map(|interpro_entry| interpro_entry.trim())
            .filter_map(|interpro_entry| {
                datastore.interpro_store().get(interpro_entry).map(|(cat, ipr)| InterproEntry {
                    code: interpro_entry.to_string(),
                    name: ipr.clone(),
                    category: cat.clone()
                })
            })
            .collect())
    }
);
