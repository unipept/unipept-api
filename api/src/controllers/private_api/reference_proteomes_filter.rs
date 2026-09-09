use std::convert::Infallible;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        generate_handlers,
        private_api::{default_sort_descending, reversed_when_descending},
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

fn get_taxon_name_by_id(taxon_store: &datastore::TaxonStore, taxon_id: u32) -> String {
    taxon_store.get_name(taxon_id).cloned().unwrap_or_else(|| "Unknown".to_string())
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    ReferenceProteomeCountParameters { filter }: ReferenceProteomeCountParameters
) -> Result<ReferenceProteomeCountResult, Infallible> {
    let proteome_store = datastore.reference_proteome_store();

    if filter.is_empty() {
        Ok(ReferenceProteomeCountResult { count: proteome_store.mapper.values().count() as u32 })
    } else {
        Ok(ReferenceProteomeCountResult {
            count: proteome_store
                .mapper
                .iter()
                .filter(|(key, (taxon_id, _, _))| {
                    let taxon_name = get_taxon_name_by_id(datastore.taxon_store(), *taxon_id);

                    key.to_lowercase().contains(&filter.to_lowercase())
                        || taxon_id.to_string().contains(&filter)
                        || taxon_name.to_lowercase().contains(&filter.to_lowercase())
                })
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

    let mut filtered_proteomes: Vec<(&String, &(u32, u32, String))> = proteome_store
        .mapper
        .iter()
        .filter(|(key, (taxon_id, _, _))| {
            let taxon_name = get_taxon_name_by_id(datastore.taxon_store(), *taxon_id);

            key.to_lowercase().contains(&filter.to_lowercase())
                || taxon_id.to_string().contains(&filter)
                || taxon_name.to_lowercase().contains(&filter.to_lowercase())
        })
        .collect();

    // A taxon name and a protein count are both held by many proteomes, and the proteome id breaks
    // every tie, so the order is total. That matters here more than in a plain listing: this list is
    // paged through, and two pages cut out of two different orders can repeat one proteome and drop
    // another.
    //
    // `sort_descending` reverses the whole ordering, tiebreak included, so a descending page is the
    // reverse of the ascending one.

    match sort_by.as_str() {
        "taxon_name" => filtered_proteomes.sort_by(|(a_id, (a_taxon_id, _, _)), (b_id, (b_taxon_id, _, _))| {
            let a_name = get_taxon_name_by_id(datastore.taxon_store(), *a_taxon_id);
            let b_name = get_taxon_name_by_id(datastore.taxon_store(), *b_taxon_id);
            reversed_when_descending((a_name, a_id).cmp(&(b_name, b_id)), sort_descending)
        }),
        "protein_count" => filtered_proteomes.sort_by(|(a_id, (_, a_count, _)), (b_id, (_, b_count, _))| {
            reversed_when_descending((a_count, a_id).cmp(&(b_count, b_id)), sort_descending)
        }),
        // A proteome id is unique, so it is a total order on its own.
        _ => {
            filtered_proteomes.sort_by(|(a_id, _), (b_id, _)| reversed_when_descending(a_id.cmp(b_id), sort_descending))
        }
    }

    // Take the range [start, end), which is empty when `end` is not past `start`.
    Ok(filtered_proteomes
        .into_iter()
        .skip(start)
        .take(end - start)
        .map(|(key, _)| key.to_string())
        .collect())
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
