use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::GoState;

#[derive(Serialize, Deserialize)]
pub struct Body {
    goterms: Vec<String>
}

#[derive(Serialize)]
pub struct GoTerm {
    code: String,
    name: String,
    namespace: String
}

pub async fn handler(
    State(GoState { go_terms }): State<GoState>,
    data: Json<Body>
) -> Json<Vec<GoTerm>> {
    Json(data.goterms
        .iter()
        .map(|go_term| go_term.trim())
        .filter_map(|go_term| {
            go_terms.get(go_term).map(|(ns, go)| GoTerm {
                code: go_term.to_string(),
                name: go.clone(),
                namespace: ns.clone()
            })
        })
        .collect()
    )
}
