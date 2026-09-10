use std::convert::Infallible;

use axum::{Json, extract::State};
use datastore::TaxonRank;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        generate_handlers,
        private_api::{default_sort_descending, page_of},
        request::Flag
    },
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
fn matches(filter: &str, taxon_id: u32, name: &str, rank: &TaxonRank, is_valid: bool) -> bool {
    if !is_valid || *rank == TaxonRank::NO_RANK {
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

    let filtered_taxa = taxon_store
        .mapper
        .iter()
        .filter(|(taxon_id, (name, rank, is_valid))| matches(&filter, **taxon_id, name, rank, *is_valid));

    // A name and a rank are both held by many taxa, and the taxon id breaks every tie, so the order
    // is total. That matters here more than in a plain listing: this list is paged through, and two
    // pages cut out of two different orders can repeat one taxon and drop another.
    //
    // `sort_descending` reverses the whole ordering, tiebreak included, so a descending page is the
    // reverse of the ascending one.
    //
    // The sort key is carried beside the id rather than read through `mapper` while sorting, which
    // would repeat that lookup for every comparison rather than doing it once per taxon.
    //
    // Each arm keeps its own row type. Sorting by id is the default, and giving it the tuple the
    // other two need would carry an empty key over every taxon in the table.
    let page = match sort_by.as_str() {
        "name" => {
            let mut rows: Vec<(&str, u32)> = filtered_taxa.map(|(id, (name, _, _))| (name.as_str(), *id)).collect();
            page_of(&mut rows, start, end, sort_descending).iter().map(|(_, id)| *id).collect()
        }
        "rank" => {
            let mut rows: Vec<(&str, u32)> = filtered_taxa.map(|(id, (_, rank, _))| (rank.as_str(), *id)).collect();
            page_of(&mut rows, start, end, sort_descending).iter().map(|(_, id)| *id).collect()
        }
        // An id is unique, so it is a total order on its own.
        _ => {
            let mut rows: Vec<u32> = filtered_taxa.map(|(id, _)| *id).collect();
            page_of(&mut rows, start, end, sort_descending).to_vec()
        }
    };

    Ok(page)
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
