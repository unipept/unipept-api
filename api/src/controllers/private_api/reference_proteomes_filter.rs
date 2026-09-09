use std::convert::Infallible;

use axum::{Json, extract::State};
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
    sort_by: String, // Can be "id", "name", or "rank"
    #[serde(default = "default_sort_descending")]
    sort_descending: Flag
}

#[derive(Serialize)]
pub struct ReferenceProteomeCountResult {
    count: u32
}

/// A proteome is kept when the filter appears in its accession, its taxon id, or its taxon name.
///
/// `lowercased` is the filter folded once by the caller rather than per proteome; `filter` as given
/// is what the taxon id is matched against, which is numeric and has no case.
fn matches(lowercased: &str, filter: &str, key: &str, taxon_id: u32, taxon_store: &datastore::TaxonStore) -> bool {
    // An empty filter keeps every proteome, and answering that before folding a case saves an
    // allocation per row on what the browser asks for by default.
    filter.is_empty()
        || key.to_lowercase().contains(lowercased)
        || taxon_id.to_string().contains(filter)
        || get_taxon_name_by_id(taxon_store, taxon_id).to_lowercase().contains(lowercased)
}

/// Borrowed rather than cloned: this is read once per proteome by the filter and once more to sort,
/// and the store owns the name for as long as either needs it.
fn get_taxon_name_by_id(taxon_store: &datastore::TaxonStore, taxon_id: u32) -> &str {
    taxon_store.get_name(taxon_id).map(String::as_str).unwrap_or("Unknown")
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    ReferenceProteomeCountParameters { filter }: ReferenceProteomeCountParameters
) -> Result<ReferenceProteomeCountResult, Infallible> {
    let proteome_store = datastore.reference_proteome_store();

    if filter.is_empty() {
        Ok(ReferenceProteomeCountResult { count: proteome_store.mapper.values().count() as u32 })
    } else {
        let lowercased = filter.to_lowercase();
        let taxon_store = datastore.taxon_store();

        Ok(ReferenceProteomeCountResult {
            count: proteome_store
                .mapper
                .iter()
                .filter(|(key, (taxon_id, _, _))| matches(&lowercased, &filter, key, *taxon_id, taxon_store))
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
        sort_by,
        sort_descending: Flag(sort_descending)
    }: ReferenceProteomeFilterParameters
) -> Result<Vec<String>, ApiError> {
    if end < start {
        return Err(ApiError::InvalidParameter(format!("end ({end}) must be at least start ({start})")));
    }

    let proteome_store = datastore.reference_proteome_store();

    let lowercased = filter.to_lowercase();
    let taxon_store = datastore.taxon_store();

    let filtered_proteomes = proteome_store
        .mapper
        .iter()
        .filter(|(key, (taxon_id, _, _))| matches(&lowercased, &filter, key, *taxon_id, taxon_store));

    // A taxon name and a protein count are both held by many proteomes, and the proteome id breaks
    // every tie, so the order is total. That matters here more than in a plain listing: this list is
    // paged through, and two pages cut out of two different orders can repeat one proteome and drop
    // another.
    //
    // `sort_descending` reverses the whole ordering, tiebreak included, so a descending page is the
    // reverse of the ascending one.

    // The sort key is carried beside the id rather than read from the taxon store while sorting,
    // which would repeat that lookup for every comparison rather than doing it once per proteome.
    let page = match sort_by.as_str() {
        "taxon_name" => {
            let mut rows: Vec<(&str, &String)> = filtered_proteomes
                .map(|(key, (taxon_id, _, _))| (get_taxon_name_by_id(taxon_store, *taxon_id), key))
                .collect();
            page_of(&mut rows, start, end, sort_descending).iter().map(|(_, key)| key.to_string()).collect()
        }
        "protein_count" => {
            let mut rows: Vec<(u32, &String)> =
                filtered_proteomes.map(|(key, (_, protein_count, _))| (*protein_count, key)).collect();
            page_of(&mut rows, start, end, sort_descending).iter().map(|(_, key)| key.to_string()).collect()
        }
        // A proteome id is unique, so it is a total order on its own.
        _ => {
            let mut rows: Vec<&String> = filtered_proteomes.map(|(key, _)| key).collect();
            page_of(&mut rows, start, end, sort_descending).iter().map(|key| key.to_string()).collect()
        }
    };

    Ok(page)
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => ReferenceProteomeCountParameters
    ) -> Result<Json<ReferenceProteomeCountResult>, Infallible> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => ReferenceProteomeFilterParameters
    ) -> Result<Json<Vec<String>>, ApiError> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
