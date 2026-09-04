use std::convert::Infallible;
use axum::{extract::State, Json};
use serde::Serialize;

use crate::{controllers::generate_handlers, AppState};

#[derive(Serialize)]
pub struct Version {
    db_version: String
}

async fn handler(State(AppState { datastore, .. }): State<AppState>) -> Result<Version, Infallible> {
    Ok(Version { db_version: datastore.version().to_string() })
}

generate_handlers!(
    async fn json_handler(state => State<AppState>) -> Result<Json<Version>, Infallible> {
        Ok(Json(handler(state).await?))
    }
);
