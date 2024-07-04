use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::generate_handlers, AppState};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    ecnumbers: Vec<String>
}

#[derive(Serialize)]
pub struct EcNumber {
    code: String,
    name: String
}

generate_handlers!(
    async fn handler(
        State(AppState { datastore, .. }): State<AppState>,
        Parameters { ecnumbers } => Parameters
    ) -> Vec<EcNumber> {
        ecnumbers
            .iter()
            .map(|ec_number| ec_number.trim())
            .filter_map(|ec_number| {
                datastore.ec_store().get(ec_number).map(|ec| EcNumber {
                    code: ec_number.to_string(),
                    name: ec.clone()
                })
            })
            .collect()
    }
);
