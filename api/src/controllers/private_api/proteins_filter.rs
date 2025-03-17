use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use database::{get_accessions_by_filter, get_accessions_count_by_filter};
use crate::{
    controllers::generate_handlers,
    AppState
};
use crate::errors::ApiError;

fn default_filter() -> String {
    String::from("")
}

fn default_sort_by() -> String { 
    String::from("uniprot_accession_id") 
}

#[derive(Deserialize)]
pub struct ProteinCountParameters {
    #[serde(default = "default_filter")]
    filter: String
}

#[derive(Deserialize)]
pub struct ProteinFilterParameters {
    #[serde(default = "default_filter")]
    filter: String,
    start: usize,
    end: usize,
    #[serde(default = "default_sort_by")]
    sort_by: String,
    #[serde(default)]
    sort_descending: bool
}

#[derive(Serialize)]
pub struct ProteinCountResult {
    count: u32
}

async fn count_handler(
    State(AppState { database, .. }): State<AppState>,
    ProteinCountParameters { filter }:  ProteinCountParameters
) -> Result<ProteinCountResult, ApiError> {
    let connection = database.get_conn().await?;
    Ok(ProteinCountResult { count: connection.interact(move |conn| get_accessions_count_by_filter(conn, filter)).await?? })
}

async fn filter_handler(
    State(AppState { database, .. }): State<AppState>,
    ProteinFilterParameters { filter, start, end, sort_by, sort_descending }:  ProteinFilterParameters
) -> Result<Vec<String>, ApiError> {
    let connection = database.get_conn().await?;
    Ok(connection.interact(move |conn| get_accessions_by_filter(conn, filter, start, end, sort_by, sort_descending)).await??)
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => ProteinCountParameters
    ) -> Result<Json<ProteinCountResult>, ApiError> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => ProteinFilterParameters
    ) -> Result<Json<Vec<String>>, ApiError> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
