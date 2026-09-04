use std::convert::Infallible;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{AppState, controllers::generate_handlers};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    #[serde(default)]
    goterms: Vec<String>
}

#[derive(Serialize)]
pub struct GoTerm {
    code: String,
    name: String,
    namespace: String
}

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { goterms }: Parameters
) -> Result<Vec<GoTerm>, Infallible> {
    Ok(goterms
        .iter()
        .map(|go_term| go_term.trim())
        .filter_map(|go_term| {
            datastore.go_store().get(go_term).map(|(ns, go)| GoTerm {
                code: go_term.to_string(),
                name: go.clone(),
                namespace: ns.clone()
            })
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<GoTerm>>, Infallible> {
        Ok(Json(handler(state, params).await?))
    }
);
