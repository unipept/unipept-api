use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use datastore::LineageRank;
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
    filter: String,
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
pub struct TaxonCountResult {
    count: u32,
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    TaxaCountParameters { filter }: TaxaCountParameters,
) -> Result<TaxonCountResult, ()> {
    let taxon_store = datastore.taxon_store();

    if filter.is_empty() {
        Ok(TaxonCountResult {
            count: taxon_store.mapper
                .values()
                .filter(|&(_, rank, is_valid)| *is_valid && *rank != LineageRank::NoRank)
                .count() as u32,
        })
    } else {
        Ok(TaxonCountResult {
            count: taxon_store.mapper
                .values()
                .filter(|&(name, rank, is_valid)| *is_valid && *rank != LineageRank::NoRank && name.to_lowercase().contains(&filter.to_lowercase()))
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
) -> Result<Vec<u32>, ()> {
    let taxon_store = datastore.taxon_store();

    let mut filtered_taxa: Vec<_> = taxon_store.mapper
        .iter()
        .filter(|(_, (name, rank, is_valid))| *is_valid && *rank != LineageRank::NoRank && name.to_lowercase().contains(&filter.to_lowercase()))
        .map(|(id, _)| *id)
        .collect();

    // Sort based on the `sort_by` field
    match sort_by.as_str() {
        "name" => {
            if sort_descending {
                filtered_taxa.sort_by(|&a_id, &b_id| {
                    taxon_store.mapper[&b_id].0.cmp(&taxon_store.mapper[&a_id].0)
                });
            } else {
                filtered_taxa.sort_by(|&a_id, &b_id| {
                    taxon_store.mapper[&a_id].0.cmp(&taxon_store.mapper[&b_id].0)
                });
            }
        }
        "rank" => {
            if sort_descending {
                filtered_taxa.sort_by(|&a_id, &b_id| {
                    LineageRank::to_string(&taxon_store.mapper[&b_id].1).cmp(&LineageRank::to_string(&taxon_store.mapper[&a_id].1))
                });
            } else {
                filtered_taxa.sort_by(|&a_id, &b_id| {
                    LineageRank::to_string(&taxon_store.mapper[&a_id].1).cmp(&LineageRank::to_string(&taxon_store.mapper[&b_id].1))
                });
            }
        }
        _ => {
            // Default to sorting by id
            if sort_descending {
                filtered_taxa.sort_by(|a, b| b.cmp(a));
            } else {
                filtered_taxa.sort_by(|a, b| a.cmp(b));
            }
        }
    }

    // Take the range [start, end)
    let taxa: Vec<u32> = filtered_taxa.into_iter().skip(start).take(end - start).collect();

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
    ) -> Result<Json<Vec<u32>>, ()> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
