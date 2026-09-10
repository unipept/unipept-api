use std::{collections::HashMap, time::Duration};

pub use errors::DatabaseError;
use models::UniprotEntry;
use opensearch::{
    OpenSearch, SearchParts,
    http::{
        Url,
        transport::{SingleNodeConnectionPool, TransportBuilder}
    }
};
use serde_json::json;

use crate::DatabaseError::GeneralError;

mod errors;
mod models;

const OPENSEARCH_TIMEOUT_DURATION: u64 = 120;

pub struct Database {
    client: OpenSearch
}

impl Database {
    pub fn try_from_url(url: &str) -> Result<Self, DatabaseError> {
        let url = Url::parse(url)?;
        let conn_pool = SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(conn_pool)
            .timeout(Duration::from_secs(OPENSEARCH_TIMEOUT_DURATION))
            .disable_proxy()
            .build()?;
        let client = OpenSearch::new(transport);
        Ok(Self { client })
    }

    pub fn get_conn(&self) -> &OpenSearch {
        &self.client
    }
}

/// Retrieves protein information from the database for a given list of UniProt accession IDs
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `accessions` - UniProt accession IDs to retrieve data for, in the order the answer should
///   take. An id the database does not hold is left out rather than reported. Deduplicate before
///   calling: an id given twice is asked for twice and answered twice, where a set could not have
///   expressed the repeat.
///
/// # Returns
/// * Vector of `UniprotEntry` records containing protein info from the database, in the order the
///   accessions were given
/// * `DatabaseError` if the database operation fails
pub async fn get_accessions(client: &OpenSearch, accessions: &[String]) -> Result<Vec<UniprotEntry>, DatabaseError> {
    if accessions.is_empty() {
        return Ok(vec![]);
    }

    let mut result: Vec<UniprotEntry> = Vec::new();

    let docs: Vec<_> = accessions.iter().map(|id| json!({ "_id": id })).collect();

    let body = json!({ "docs": docs });

    let response = client.mget(opensearch::MgetParts::Index("uniprot_entries")).body(body).send().await?;

    if response.status_code().is_success() {
        let response_body: serde_json::Value = response.json().await?;

        if let Some(docs) = response_body.get("docs").and_then(|docs| docs.as_array()) {
            for doc in docs {
                if let Some(source) = doc.get("_source")
                    && let Ok(entry) = serde_json::from_value::<UniprotEntry>(source.clone())
                {
                    result.push(entry);
                }
            }
        }
    } else {
        return Err(GeneralError(response.text().await?));
    }

    Ok(result)
}

/// Gets protein information as a map with UniProt accession IDs as keys and UniprotEntry objects as values
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `accessions` - UniProt accession IDs to retrieve data for
///
/// # Returns
/// * HashMap mapping UniProt accession IDs to their corresponding UniprotEntry records
/// * `DatabaseError` if the database operation fails
///
/// This function returns the same protein information as `get_accessions()` but organized as a lookup map
/// instead of a vector, allowing direct access to entries by their accession ID.
pub async fn get_accessions_map(
    client: &OpenSearch,
    accessions: &[String]
) -> Result<HashMap<String, UniprotEntry>, DatabaseError> {
    Ok(get_accessions(client, accessions)
        .await?
        .into_iter()
        .map(|entry| (entry.uniprot_accession_number.clone(), entry))
        .collect())
}

/// Counts the number of UniProt entries in the database that match the given filter string.
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `filter` - String to filter entries by. If empty, returns total count of all entries
///
/// # Returns
/// * Number of matching entries (as u32)
/// * `DatabaseError` if the database operation fails
///
/// This function counts UniProt entries where either:
/// - Entry name contains the filter string (case-insensitive)
/// - UniProt accession number contains the filter string
/// - Taxon ID contains the filter number (if filter is a valid integer, discarded otherwise)
pub async fn get_accessions_count_by_filter(client: &OpenSearch, filter: String) -> Result<u32, DatabaseError> {
    // If filter is empty, use match_all query to count all documents
    if filter.is_empty() {
        let body = json!({
            "query": {
                "match_all": {}
            },
            "track_total_hits": true
        });

        let response = client
            .search(SearchParts::Index(&["uniprot_entries"]))
            .size(0) // We only need count, no actual documents
            .body(body)
            .send()
            .await?;

        if !response.status_code().is_success() {
            return Err(GeneralError(response.text().await?));
        }

        let response_body: serde_json::Value = response.json().await?;
        return Ok(response_body["hits"]["total"]["value"].as_u64().unwrap_or(0) as u32);
    }

    // Parse filter as integer for taxon_id matching if possible
    let taxon_filter = filter.parse::<u32>().ok();

    let mut should_conditions = vec![
        // Name contains filter
        json!({
            "wildcard": {
                "name": {
                    "value": format!("*{}*", filter),
                    "case_insensitive": true
                }
            }
        }),
        // Uniprot accession number contains filter
        json!({
            "prefix": {
                "uniprot_accession_number": {
                    "value": filter,
                    "case_insensitive": true
                }
            }
        }),
    ];

    // Add taxon_id term query if filter is a valid integer
    if let Some(taxon_id) = taxon_filter {
        should_conditions.push(json!({
            "match": {
                "taxon_id": {
                    "query": taxon_id
                }
            }
        }));
    }

    let body = json!({
        "query": {
            "bool": {
                "should": should_conditions,
                "minimum_should_match": 1
            }
        },
        "track_total_hits": true
    });

    let response = client
        .search(SearchParts::Index(&["uniprot_entries"]))
        .size(0) // We only need count, no actual documents
        .body(body)
        .send()
        .await?;

    if !response.status_code().is_success() {
        return Err(GeneralError(response.text().await?));
    }

    let response_body: serde_json::Value = response.json().await?;

    Ok(response_body["hits"]["total"]["value"].as_u64().unwrap_or(0) as u32)
}

/// A field the `uniprot_entries` mapping can be sorted on.
///
/// Only `keyword` and numeric fields carry doc values, and sorting needs them. `name` is mapped
/// `text`, with no `.keyword` subfield, so a sort on it is refused by the cluster rather than being
/// slow — which is why this is a type and not a string: the API cannot hand over a field the
/// mapping will not sort.
///
/// `sequence` and `fa` are `text` with `index: false`, so they are neither searchable nor sortable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProteinSortField {
    /// Unique, so it orders a page on its own and breaks every tie below.
    Accession,
    TaxonId,
    DbType
}

impl ProteinSortField {
    /// The name the field carries in the mapping.
    ///
    /// `DbType` is `type` in the index and `db_type` on the wire, because `type` is reserved in the
    /// clients that read this.
    fn as_mapping_field(self) -> &'static str {
        match self {
            ProteinSortField::Accession => "uniprot_accession_number",
            ProteinSortField::TaxonId => "taxon_id",
            ProteinSortField::DbType => "type"
        }
    }
}

/// The sort clause for a listing, as a total order.
///
/// A taxon id and a db type are each held by millions of entries, so neither orders a page on its
/// own: two pages cut out of two different orders can repeat one entry and drop another. The
/// accession breaks every tie, and is unique, so the order is total.
///
/// The tiebreak is reversed along with the primary field, so a descending page is the exact reverse
/// of the ascending one — the same reading `page_of` takes for the in-memory listings.
fn sort_clause(sort_by: ProteinSortField, descending: bool) -> serde_json::Value {
    let order = if descending { "desc" } else { "asc" };
    let mut clauses = vec![json!({ sort_by.as_mapping_field(): { "order": order } })];

    if sort_by != ProteinSortField::Accession {
        clauses.push(json!({ ProteinSortField::Accession.as_mapping_field(): { "order": order } }));
    }

    json!(clauses)
}

/// How deep a `from`/`size` search may reach.
///
/// OpenSearch refuses a search whose `from + size` passes `index.max_result_window`, which the
/// cluster leaves at its default. The refusal is a 400 from the cluster, which this crate reports
/// as a `GeneralError` and the API answers as a 500 — so a caller has to be stopped before the
/// query is sent, not after.
pub const MAX_RESULT_WINDOW: usize = 10_000;

/// A paging bound as OpenSearch takes it.
///
/// `from` and `size` travel as `i64`, and both are computed from `usize` values a caller controls.
/// Saturating rather than casting: a `usize` above `i64::MAX` wraps to a negative number, and the
/// subtraction underflows when `end` is below `start`. Either sends the cluster a bound it refuses,
/// which surfaces as a 500 for what is a malformed request.
fn as_window_bound(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Gets UniProt accession IDs from the database that match the given filter criteria
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `filter` - String to filter entries by. If empty, returns unfiltered results
/// * `start` - Starting index for pagination
/// * `end` - Ending index for pagination
///
/// # Returns
/// * Vector of UniProt accession IDs that match the filter criteria
/// * `DatabaseError` if the database operation fails
///
/// This function returns UniProt accession IDs where either:
/// - Entry name contains the filter string (case-insensitive)
/// - UniProt accession number contains the filter string
/// - Taxon ID contains the filter number (if filter is a valid integer, discarded otherwise)
#[allow(clippy::needless_late_init)]
pub async fn get_accessions_by_filter(
    client: &OpenSearch,
    filter: String,
    start: usize,
    end: usize,
    sort_by: ProteinSortField,
    sort_descending: bool
) -> Result<Vec<String>, DatabaseError> {
    let body;

    // If filter is empty, use match_all query to count all documents
    if filter.is_empty() {
        body = json!({
            "query": {
                "match_all": {}
            },
            "sort": sort_clause(sort_by, sort_descending)
        });
    } else {
        // Parse filter as integer for taxon_id matching if possible
        let taxon_filter = filter.parse::<u32>().ok();

        let mut should_conditions = vec![
            // Name contains filter
            json!({
            "wildcard": {
                "name": {
                    "value": format!("*{}*", filter),
                    "case_insensitive": true
                }
            }
            }),
            // Uniprot accession number contains filter
            json!({
                "prefix": {
                    "uniprot_accession_number": {
                        "value": filter,
                        "case_insensitive": true
                    }
                }
            }),
        ];

        // Add taxon_id term query if filter is a valid integer
        if let Some(taxon_id) = taxon_filter {
            should_conditions.push(json!({
                "term": {
                    "taxon_id": taxon_id
                }
            }));
        }

        body = json!({
            "query": {
                "bool": {
                    "should": should_conditions,
                    "minimum_should_match": 1
                }
            },
            "sort": sort_clause(sort_by, sort_descending)
        });
    }

    let response = client
        .search(SearchParts::Index(&["uniprot_entries"]))
        .from(as_window_bound(start))
        .size(as_window_bound(end.saturating_sub(start)))
        .body(body)
        .send()
        .await?;

    if !response.status_code().is_success() {
        return Err(GeneralError(response.text().await?));
    }

    let response_body: serde_json::Value = response.json().await?;

    Ok(response_body["hits"]["hits"]
        .as_array()
        .map(|hits| {
            hits.iter()
                .filter_map(|hit| hit["_source"]["uniprot_accession_number"].as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default())
}
