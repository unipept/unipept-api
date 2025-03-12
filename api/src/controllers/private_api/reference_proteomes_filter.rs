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

#[derive(Deserialize)]
pub struct ReferenceProteomeFilterParameters {
    #[serde(default = "default_filter")]
    filter: String,
    start: usize,
    end: usize,
    #[serde(default)]
    sort_descending: bool
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

async fn filter_handler(
    State(AppState { datastore, .. }): State<AppState>,
    ReferenceProteomeFilterParameters {
        filter,
        start,
        end,
        sort_descending
    }: ReferenceProteomeFilterParameters
) -> Result<Vec<String>, ()> {
    let proteome_store = datastore.reference_proteome_store();

    let mut filtered_proteomes: Vec<_> = proteome_store.mapper
        .keys()
        .filter(|key| key.to_lowercase().contains(&filter.to_lowercase()))
        .map(|key| key.to_string())
        .collect();

    if sort_descending {
        filtered_proteomes.sort_by(|a, b| b.cmp(a));
    } else {
        filtered_proteomes.sort();
    }

    Ok(filtered_proteomes.into_iter().skip(start).take(end - start).collect())
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => ReferenceProteomeCountParameters
    ) -> Result<Json<ReferenceProteomeCountResult>, ()> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => ReferenceProteomeFilterParameters
    ) -> Result<Json<Vec<String>>, ()> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
