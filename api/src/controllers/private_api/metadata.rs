use axum::{extract::State, Json};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct Version {
    db_version: String
}

pub async fn handler(
    State(AppState { datastore }): State<AppState>
) -> Json<Version> {
    Json(Version {
        db_version: datastore.version().to_string()
    })
}
