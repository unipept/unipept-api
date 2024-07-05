use axum::{
    extract::State,
    Json
};
use datastore::SampleStore;

use crate::{
    controllers::generate_handlers,
    AppState
};

async fn handler(
    State(AppState {
        datastore, ..
    }): State<AppState>
) -> Result<SampleStore, ()> {
    Ok(datastore.sample_store().to_owned())
}

generate_handlers!(
    async fn json_handler(state => State<AppState>) -> Result<Json<SampleStore>, ()> {
        Ok(Json(handler(state).await?))
    }
);
