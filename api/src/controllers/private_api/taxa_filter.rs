use axum::{extract::State, Json};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    AppState
};

fn default_filter() -> String {
    String::from("")
}

#[derive(Deserialize)]
pub struct TaxaCountParameters {
    #[serde(default = "default_filter")]
    filter: String
}

#[derive(Deserialize)]
pub struct TaxaFilterParameters {
    #[serde(default = "default_filter")]
    filter: String,
    start: usize,
    end: usize,
    #[serde(default)]
    sort_by: String, // Can be "id", "name", or "rank"
    #[serde(default)]
    sort_descending: bool,
}

#[derive(Serialize)]
pub struct Taxon {
    id: u32,
    name: String,
    rank: String,
    lineage: Vec<Option<i32>>
}

#[derive(Serialize)]
pub struct TaxonCountResult {
    count: u32
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    TaxaCountParameters { filter }: TaxaCountParameters,
) -> Result<TaxonCountResult, ()> {
    let taxon_store = datastore.taxon_store();

    if (filter == "") {
        Ok(TaxonCountResult {
            count: taxon_store.mapper.len() as u32
        })
    } else {
        Ok(TaxonCountResult {
            count: taxon_store.mapper
                .values()
                .filter(|&(name, _, _)| name.to_lowercase().contains(&filter.to_lowercase()))
                .count() as u32,
        })
    }
}

async fn filter_handler(
    State(AppState { datastore, .. }): State<AppState>,
    TaxaFilterParameters {
        filter,
        start,
        end,
        sort_by,
        sort_descending,
    }: TaxaFilterParameters,
) -> Result<Vec<Taxon>, ()> {
    let taxon_store = datastore.taxon_store();

    let mut filtered_taxa: Vec<_> = taxon_store.mapper
        .iter()
        .filter(|(_, (name, _, _))| name.to_lowercase().contains(&filter.to_lowercase()))
        .map(|(id, (name, rank, _))| Taxon {
            id: *id,
            name: name.clone(),
            rank: rank.to_string(),
            lineage: vec![], // Assuming lineage information is not present in the provided code
        })
        .collect();

    // Sort based on the `sort_by` field
    match sort_by.as_str() {
        "name" => {
            if sort_descending {
                filtered_taxa.sort_by(|a, b| b.name.cmp(&a.name));
            } else {
                filtered_taxa.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
        "rank" => {
            if sort_descending {
                filtered_taxa.sort_by(|a, b| b.rank.cmp(&a.rank));
            } else {
                filtered_taxa.sort_by(|a, b| a.rank.cmp(&b.rank));
            }
        }
        _ => {
            // Default to sorting by id
            if sort_descending {
                filtered_taxa.sort_by(|a, b| b.id.cmp(&a.id));
            } else {
                filtered_taxa.sort_by(|a, b| a.id.cmp(&b.id));
            }
        }
    }

    // Take the range [start, end)
    let taxa: Vec<_> = filtered_taxa.into_iter().skip(start).take(end - start).collect();

    Ok(taxa)
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => TaxaCountParameters
    ) -> Result<Json<TaxonCountResult>, ()> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => TaxaFilterParameters
    ) -> Result<Json<Vec<Taxon>>, ()> {
        Ok(Json(filter_handler(state, params).await?))
    }
);


