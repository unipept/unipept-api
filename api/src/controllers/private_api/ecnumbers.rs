use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct Body {
    ecnumbers: Vec<String>
}

#[derive(Serialize)]
pub struct EcNumber {
    code: String,
    name: String
}

pub async fn handler(
    State(AppState { datastore }): State<AppState>,
    data: Json<Body>
) -> Json<Vec<EcNumber>> {
    Json(data.ecnumbers
        .iter()
        .map(|ec_number| ec_number.trim())
        .filter_map(|ec_number| {
            datastore.ec_store().get(ec_number).map(|ec| EcNumber {
                code: ec_number.to_string(),
                name: ec.clone()
            })
        })
        .collect()
    )
}
