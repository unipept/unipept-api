use std::convert::Infallible;

use axum::{Json, extract::State};
use datastore::SampleStore;

use crate::{AppState, controllers::generate_handlers};

async fn handler(State(AppState { datastore, .. }): State<AppState>) -> Result<SampleStore, Infallible> {
    Ok(datastore.sample_store().to_owned())
}

generate_handlers!(
    async fn json_handler(state => State<AppState>) -> Result<Json<SampleStore>, Infallible> {
        Ok(Json(handler(state).await?))
    }
);
