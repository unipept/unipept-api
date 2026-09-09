use std::{cmp::Ordering, convert::Infallible};

use axum::{Json, extract::State};
use datastore::LineageRank;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{generate_handlers, private_api::default_sort_descending, request::Flag},
    errors::ApiError
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
/// once per taxon. An empty filter takes every ranked, valid taxon without lowercasing its name,
/// which is the whole taxon table on the counting path.
///
/// The rank is matched as `as_str` spells it, with a space, because that is the name the response
/// carries and so the name a caller reads. The `descendants_ranks` parameter of `/api/v2/taxonomy`
/// takes the underscore spelling instead, since it names a lineage column.
fn matches(filter: &str, taxon_id: u32, name: &str, rank: &LineageRank, is_valid: bool) -> bool {
    if !is_valid || *rank == LineageRank::NoRank {
        return false;
    }

    filter.is_empty()
        || name.to_lowercase().contains(filter)
        || taxon_id.to_string().contains(filter)
        || rank.as_str().contains(filter)
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
) -> Result<Vec<u32>, ApiError> {
    if end < start {
        return Err(ApiError::InvalidParameter(format!("end ({end}) must be at least start ({start})")));
    }

    let taxon_store = datastore.taxon_store();

    let filter = filter.to_lowercase();

    let mut filtered_taxa: Vec<_> = taxon_store
        .mapper
        .iter()
        .filter(|(taxon_id, (name, rank, is_valid))| matches(&filter, **taxon_id, name, rank, *is_valid))
        .map(|(id, _)| *id)
        .collect();

    // A name and a rank are both held by many taxa, and the taxon id breaks every tie, so the order
    // is total. That matters here more than in a plain listing: this list is paged through, and two
    // pages cut out of two different orders can repeat one taxon and drop another.
    //
    // `sort_descending` reverses the whole ordering, tiebreak included, so a descending page is the
    // reverse of the ascending one.
    let reversed_when_descending = |ordering: Ordering| if sort_descending { ordering.reverse() } else { ordering };

    match sort_by.as_str() {
        "name" => filtered_taxa.sort_by(|a, b| {
            reversed_when_descending((&taxon_store.mapper[a].0, a).cmp(&(&taxon_store.mapper[b].0, b)))
        }),
        "rank" => filtered_taxa.sort_by(|a, b| {
            reversed_when_descending((&taxon_store.mapper[a].1, a).cmp(&(&taxon_store.mapper[b].1, b)))
        }),
        // An id is unique, so it is a total order on its own.
        _ => filtered_taxa.sort_by(|a, b| reversed_when_descending(a.cmp(b)))
    }

    // Take the range [start, end), which is empty when `end` is not past `start`.
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
    ) -> Result<Json<Vec<u32>>, ApiError> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
