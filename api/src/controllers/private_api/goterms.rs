use axum::{
    extract::State,
    Json
};
use serde::{
    Deserialize,
    Serialize
};

use crate::{
    controllers::generate_handlers,
    AppState
};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    goterms: Vec<String>
}

#[derive(Serialize)]
pub struct GoTerm {
    code:      String,
    name:      String,
    namespace: String
}

async fn handler(
    State(AppState {
        datastore, ..
    }): State<AppState>,
    Parameters {
        goterms
    }: Parameters
) -> Result<Vec<GoTerm>, ()> {
    Ok(goterms
        .iter()
        .map(|go_term| go_term.trim())
        .filter_map(|go_term| {
            datastore.go_store().get(go_term).map(|(ns, go)| GoTerm {
                code:      go_term.to_string(),
                name:      go.clone(),
                namespace: ns.clone()
            })
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<GoTerm>>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
