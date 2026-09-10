use axum::{Json, extract::State};
use database::{MAX_RESULT_WINDOW, ProteinSortField, get_accessions_by_filter, get_accessions_count_by_filter};
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
pub struct ProteinCountParameters {
    #[serde(default = "default_filter")]
    filter: String
}

#[derive(Deserialize)]
pub struct ProteinFilterParameters {
    #[serde(default = "default_filter")]
    filter: String,
    start: usize,
    end: usize,
    #[serde(default)]
    sort_by: String,
    #[serde(default = "default_sort_descending")]
    sort_descending: Flag
}

/// The sort field a caller named.
///
/// Unlike the listings that sort in memory, this one cannot fall back to a default for a field it
/// does not know. `name` is mapped `text`, so the cluster refuses to sort on it — and answering a
/// page ordered by accession to a caller who asked for `name` is the silent wrong answer this
/// endpoint already gave, when the parameter was dropped altogether.
fn sort_field(sort_by: &str) -> Result<ProteinSortField, ApiError> {
    match sort_by {
        // The browser sends this by default, and an absent value means the same.
        "" | "uniprot_accession_number" => Ok(ProteinSortField::Accession),
        "taxon_id" => Ok(ProteinSortField::TaxonId),
        "db_type" | "type" => Ok(ProteinSortField::DbType),
        other => Err(ApiError::InvalidParameter(format!(
            "cannot sort on {other}: this endpoint sorts on uniprot_accession_number, taxon_id or db_type"
        )))
    }
}

#[derive(Serialize)]
pub struct ProteinCountResult {
    count: u32
}

async fn count_handler(
    State(AppState { database, .. }): State<AppState>,
    ProteinCountParameters { filter }: ProteinCountParameters
) -> Result<ProteinCountResult, ApiError> {
    let connection = database.get_conn();
    Ok(ProteinCountResult {
        count: get_accessions_count_by_filter(connection, filter).await?
    })
}

async fn filter_handler(
    State(AppState { database, .. }): State<AppState>,
    ProteinFilterParameters {
        filter,
        start,
        end,
        sort_by,
        sort_descending: Flag(sort_descending)
    }: ProteinFilterParameters
) -> Result<Vec<String>, ApiError> {
    if end < start {
        return Err(ApiError::InvalidParameter(format!("end ({end}) must be at least start ({start})")));
    }

    // The cluster refuses to page this deep, and answers a 400 the API cannot tell from a failure
    // of its own. Refusing here names the limit instead of reporting an internal error.
    if end > MAX_RESULT_WINDOW {
        return Err(ApiError::InvalidParameter(format!(
            "end ({end}) is past the {MAX_RESULT_WINDOW} entries this endpoint can page over"
        )));
    }

    let sort_by = sort_field(&sort_by)?;

    let connection = database.get_conn();
    Ok(get_accessions_by_filter(connection, filter, start, end, sort_by, sort_descending).await?)
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => ProteinCountParameters
    ) -> Result<Json<ProteinCountResult>, ApiError> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => ProteinFilterParameters
    ) -> Result<Json<Vec<String>>, ApiError> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
