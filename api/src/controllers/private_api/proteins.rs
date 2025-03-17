use std::collections::HashSet;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use database::get_accessions;
use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    accessions: Vec<String>
}

#[derive(Serialize)]
pub struct Protein {
    uniprot_accession_id: u32,
    name: String,
    taxon_id: u32,
    db_type: String
}

async fn handler(
    State(AppState { database, .. }): State<AppState>,
    Parameters { accessions }: Parameters
) -> Result<Vec<Protein>, ApiError> {
    let connection = database.get_conn().await?;

    let entries = connection.interact(move |conn| get_accessions(conn, &HashSet::from_iter(accessions.iter().cloned()))).await??;

    Ok(entries
        .into_iter()
        .map(|entry| Protein {
            uniprot_accession_id: entry.id,
            name: entry.name,
            taxon_id: entry.taxon_id,
            db_type: entry.db_type,
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<Protein>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
