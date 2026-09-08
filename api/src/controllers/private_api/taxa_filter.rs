use std::convert::Infallible;

use axum::{Json, extract::State};
use datastore::LineageRank;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{generate_handlers, private_api::default_sort_descending, request::Flag}
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
    #[serde(default = "default_sort_descending")]
    sort_descending: Flag
}

#[derive(Serialize)]
pub struct TaxonCountResult {
    count: u32
}

/// Whether a taxon belongs in a filtered listing.
///
/// `filter` is matched already lowercased, so the caller lowercases it once per request rather than
/// once per taxon. An empty filter matches every ranked, valid taxon, since `contains("")` holds.
fn matches(filter: &str, taxon_id: u32, name: &str, rank: &LineageRank, is_valid: bool) -> bool {
    is_valid
        && *rank != LineageRank::NoRank
        && (name.to_lowercase().contains(filter)
            || taxon_id.to_string().contains(filter)
            || rank.as_str().contains(filter))
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    TaxaCountParameters { filter }: TaxaCountParameters
) -> Result<TaxonCountResult, Infallible> {
    let taxon_store = datastore.taxon_store();
    let filter = filter.to_lowercase();

    Ok(TaxonCountResult {
        count: taxon_store
            .mapper
            .iter()
            .filter(|(taxon_id, (name, rank, is_valid))| matches(&filter, **taxon_id, name, rank, *is_valid))
            .count() as u32
    })
}

async fn filter_handler(
    State(AppState { datastore, .. }): State<AppState>,
    TaxaFilterParameters {
        filter,
        start,
        end,
        sort_by,
        sort_descending: Flag(sort_descending)
    }: TaxaFilterParameters
) -> Result<Vec<u32>, Infallible> {
    let taxon_store = datastore.taxon_store();

    let filter = filter.to_lowercase();

    let mut filtered_taxa: Vec<_> = taxon_store
        .mapper
        .iter()
        .filter(|(taxon_id, (name, rank, is_valid))| matches(&filter, **taxon_id, name, rank, *is_valid))
        .map(|(id, _)| *id)
        .collect();

    // Sort based on the `sort_by` field
    match sort_by.as_str() {
        "name" => {
            if sort_descending {
                filtered_taxa.sort_by(|&a_id, &b_id| taxon_store.mapper[&b_id].0.cmp(&taxon_store.mapper[&a_id].0));
            } else {
                filtered_taxa.sort_by(|&a_id, &b_id| taxon_store.mapper[&a_id].0.cmp(&taxon_store.mapper[&b_id].0));
            }
        }
        "rank" => {
            if sort_descending {
                filtered_taxa.sort_by(|&a_id, &b_id| taxon_store.mapper[&b_id].1.cmp(&taxon_store.mapper[&a_id].1));
            } else {
                filtered_taxa.sort_by(|&a_id, &b_id| taxon_store.mapper[&a_id].1.cmp(&taxon_store.mapper[&b_id].1));
            }
        }
        _ => {
            // Default to sorting by id
            if sort_descending {
                filtered_taxa.sort_by(|a, b| b.cmp(a));
            } else {
                #[allow(clippy::unnecessary_sort_by)]
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
    ) -> Result<Json<TaxonCountResult>, Infallible> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => TaxaFilterParameters
    ) -> Result<Json<Vec<u32>>, Infallible> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
