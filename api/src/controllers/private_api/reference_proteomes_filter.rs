use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::{
    controllers::generate_handlers,
    AppState
};

fn default_filter() -> String {
    String::from("")
}

#[derive(Deserialize)]
pub struct ReferenceProteomeCountParameters {
    #[serde(default = "default_filter")]
    filter: String
}

#[derive(Serialize)]
pub struct ReferenceProteomeCountResult {
    count: u32
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    ReferenceProteomeCountParameters { filter }: ReferenceProteomeCountParameters
) -> Result<ReferenceProteomeCountResult, ()> {
    let proteome_store = datastore.reference_proteome_store();
    
    if filter.is_empty() {
        Ok(ReferenceProteomeCountResult {
            count: proteome_store.mapper
                .values()
                .count() as u32
        })
    } else {
        Ok(ReferenceProteomeCountResult {
            count: proteome_store.mapper
                .keys()
                .filter(|key| key.to_lowercase().contains(&filter.to_lowercase()))
                .count() as u32
        })
    }
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => ReferenceProteomeCountParameters
    ) -> Result<Json<ReferenceProteomeCountResult>, ()> {
        Ok(Json(count_handler(state, params).await?))
    }
);
