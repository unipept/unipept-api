use axum::{
    extract::State,
    Json
};
use datastore::SampleStore;

use crate::{
    controllers::generate_json_handlers, AppState
};

generate_json_handlers!(
    async fn handler(
        State(AppState {
            datastore, ..
        }): State<AppState>
    ) -> Result<SampleStore, ()> {
        Ok(datastore.sample_store().to_owned())
    }
);
