use axum::{extract::State, Json};
use datastore::SampleStore;

use crate::{controllers::generate_handlers, AppState};

generate_handlers!(
    async fn handler(
        State(AppState { datastore, .. }): State<AppState>
    ) -> Json<SampleStore> {
        Json(datastore.sample_store().to_owned())
    }
);
