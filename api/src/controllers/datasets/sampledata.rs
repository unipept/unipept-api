use std::convert::Infallible;
use axum::{extract::State, Json};
use datastore::SampleStore;

use crate::{controllers::generate_handlers, AppState};

async fn handler(State(AppState { datastore, .. }): State<AppState>) -> Result<SampleStore, Infallible> {
    Ok(datastore.sample_store().to_owned())
}

generate_handlers!(
    async fn json_handler(state => State<AppState>) -> Result<Json<SampleStore>, Infallible> {
        Ok(Json(handler(state).await?))
    }
);
