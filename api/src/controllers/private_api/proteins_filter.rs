use axum::{Json, extract::State};
use database::{DatabaseError, get_accessions_by_filter, get_accessions_count_by_filter};
use serde::{Deserialize, Serialize};

use crate::{AppState, controllers::generate_handlers, errors::ApiError};

fn default_filter() -> String {
    String::from("")
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
    end: usize
}

#[derive(Serialize)]
pub struct ProteinCountResult {
    count: u32
}

async fn count_handler(
    State(AppState { database, .. }): State<AppState>,
    ProteinCountParameters { filter }: ProteinCountParameters
) -> Result<ProteinCountResult, ApiError> {
    let connection = database.get_conn();
    Ok(ProteinCountResult {
        count: get_accessions_count_by_filter(connection, filter).await?
    })
}

async fn filter_handler(
    State(AppState { database, .. }): State<AppState>,
    ProteinFilterParameters { filter, start, end }: ProteinFilterParameters
) -> Result<Vec<String>, ApiError> {
    if end < start {
        return Err(ApiError::InvalidParameter(format!("end ({end}) must be at least start ({start})")));
    }

    let connection = database.get_conn();
    match get_accessions_by_filter(connection, filter, start, end).await {
        Ok(page) => Ok(page),
        // A page the cluster can reach from neither end is a request that cannot be served, not a
        // fault of ours. The message says what is reachable, so a caller can ask for that instead.
        Err(error @ DatabaseError::WindowUnreachable { .. }) => Err(ApiError::InvalidParameter(error.to_string())),
        Err(error) => Err(error.into())
    }
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
