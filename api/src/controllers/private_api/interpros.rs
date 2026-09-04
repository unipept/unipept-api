use std::convert::Infallible;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{AppState, controllers::generate_handlers};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    #[serde(default)]
    interpros: Vec<String>
}

#[derive(Serialize)]
pub struct InterproEntry {
    code: String,
    name: String,
    category: String
}

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { interpros }: Parameters
) -> Result<Vec<InterproEntry>, Infallible> {
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

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<InterproEntry>>, Infallible> {
        Ok(Json(handler(state, params).await?))
    }
);
