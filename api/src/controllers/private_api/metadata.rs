use axum::{extract::State, Json};
use serde::Serialize;

use crate::{controllers::generate_json_handlers, AppState};

#[derive(Serialize)]
pub struct Version {
    db_version: String
}

generate_json_handlers!(
    async fn handler(
        State(AppState { datastore, .. }): State<AppState>
    ) -> Version {
        Version {
            db_version: datastore.version().to_string()
        }
    }
);
