use std::{collections::HashMap};
use std::collections::HashSet;
use std::time::Duration;
pub use errors::DatabaseError;
use models::UniprotEntry;
use opensearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use opensearch::http::{Url};
use opensearch::{OpenSearch, SearchParts};
use serde::Deserialize;
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

/// Retrieves protein information from the database for a given set of UniProt accession IDs
///
/// # Arguments
/// * `conn` - Database connection handle 
/// * `accessions` - Set of UniProt accession IDs to retrieve data for
///
/// # Returns
/// * Vector of `UniprotEntry` records containing protein info from the database, ordered to match
///   the order of accessions in the input set
/// * `DatabaseError` if the database operation fails
pub async fn get_accessions(
    client: &OpenSearch,
    accessions: &HashSet<String>,
) -> Result<Vec<UniprotEntry>, DatabaseError> {
    if accessions.is_empty() {
        return Ok(vec![]);
    }

    let mut result: Vec<UniprotEntry> = Vec::new();
    
    let docs: Vec<_> = accessions
        .iter()
        .map(|id| json!({ "_id": id }))
        .collect();

    let body = json!({ "docs": docs });

    let response = client
        .mget(opensearch::MgetParts::Index("uniprot_entries"))
        .body(body)
        .send()
        .await?;
    
    if response.status_code().is_success() {
        let response_body: serde_json::Value = response.json().await?;
        
        if let Some(docs) = response_body.get("docs").and_then(|docs| docs.as_array()) {
            for doc in docs {
                if let Some(source) = doc.get("_source") {
                    if let Ok(entry) = serde_json::from_value::<UniprotEntry>(source.clone()) {
                        result.push(entry);
                    }
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
/// * `accessions` - Set of UniProt accession IDs to retrieve data for
///
/// # Returns
/// * HashMap mapping UniProt accession IDs to their corresponding UniprotEntry records
/// * `DatabaseError` if the database operation fails
///
/// This function returns the same protein information as `get_accessions()` but organized as a lookup map
/// instead of a vector, allowing direct access to entries by their accession ID.
pub async fn get_accessions_map(
    client: &OpenSearch,
    accessions: &HashSet<String>,
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
pub async fn get_accessions_count_by_filter(
    client: &OpenSearch,
    filter: String,
) -> Result<u32, DatabaseError> {
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
        return Ok(response_body["hits"]["total"]["value"]
            .as_u64()
            .unwrap_or(0) as u32);
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
        })
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
    
    Ok(response_body["hits"]["total"]["value"]
        .as_u64()
        .unwrap_or(0) as u32)
}

/// Counts proteins per taxon ID for a given list of taxon IDs.
///
/// # Arguments
/// * `client` - OpenSearch client handle
/// * `taxon_ids` - Slice of taxon IDs to count proteins for
///
/// # Returns
/// * HashMap mapping each taxon ID to its protein count. Taxon IDs with no proteins are absent
///   from the map (i.e., their count is implicitly 0).
/// * `DatabaseError` if the query fails
// OpenSearch enforces a hard limit on the number of terms in a single Terms Query.
const OPENSEARCH_MAX_TERMS: usize = 65_000;

pub async fn get_protein_counts_by_taxon_ids(
    client: &OpenSearch,
    taxon_ids: &[i32],
) -> Result<HashMap<u32, u32>, DatabaseError> {
    if taxon_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let taxon_ids_positive: Vec<i32> = taxon_ids.iter().copied().filter(|&id| id > 0).collect();

    if taxon_ids_positive.is_empty() {
        return Ok(HashMap::new());
    }

    let mut merged: HashMap<u32, u32> = HashMap::new();

    for chunk in taxon_ids_positive.chunks(OPENSEARCH_MAX_TERMS) {
        let taxon_ids_json: Vec<serde_json::Value> = chunk.iter().map(|&id| json!(id)).collect();
        let size = taxon_ids_json.len();

        let body = json!({
            "query": {
                "terms": {
                    "taxon_id": taxon_ids_json
                }
            },
            "aggs": {
                "proteins_per_taxon": {
                    "terms": {
                        "field": "taxon_id",
                        "size": size
                    }
                }
            },
            "size": 0,
            "track_total_hits": false
        });

        let response = client
            .search(SearchParts::Index(&["uniprot_entries"]))
            .body(body)
            .send()
            .await?;

        if !response.status_code().is_success() {
            return Err(GeneralError(response.text().await?));
        }

        let response_body: serde_json::Value = response.json().await?;

        if let Some(buckets) = response_body["aggregations"]["proteins_per_taxon"]["buckets"].as_array() {
            for bucket in buckets {
                if let (Some(taxon_id), Some(count)) = (
                    bucket["key"].as_u64().map(|v| v as u32),
                    bucket["doc_count"].as_u64().map(|v| v as u32),
                ) {
                    *merged.entry(taxon_id).or_insert(0) += count;
                }
            }
        }
    }

    Ok(merged)
}


/// Pages through `uniprot_entries` documents for `taxon_id` using `search_after` and returns
/// the raw `_source` JSON value for each hit.
///
/// * `source_filter` — restrict which fields OpenSearch includes (e.g. `Some(&["sequence"])`);
///   `None` fetches all fields.
/// * `range` — when `Some((start, end))`, only the half-open slice `[start, end)` of the
///   stable `uniprot_accession_number`-ordered result set is collected. Pages entirely before
///   `start` are fetched with `_source: false` to minimise transferred payload (the cursor value
///   comes from `hit["sort"]`, not `_source`). Pages are not fetched beyond the first page that
///   reaches index `end`. When `None`, all documents are returned (existing behaviour).
///
/// Returns an empty `Vec` immediately when `range` is `Some((start, end))` with `start >= end`.
async fn fetch_taxon_sources(
    client: &OpenSearch,
    taxon_id: u32,
    source_filter: Option<&[&str]>,
    range: Option<(usize, usize)>,
) -> Result<Vec<serde_json::Value>, DatabaseError> {
    // Guard: empty range.
    if let Some((start, end)) = range {
        if start >= end {
            return Ok(Vec::new());
        }
    }

    const PAGE_SIZE: usize = 1000;
    let mut all_sources: Vec<serde_json::Value> = Vec::new();
    let mut search_after: Option<String> = None;
    // Running count of documents seen across all pages so far.
    let mut global_index: usize = 0;

    loop {
        let (range_start, range_end) = match range {
            Some(r) => r,
            // No range filter — fetch everything as before.
            None => (0, usize::MAX),
        };

        // For pages that fall entirely before range_start, suppress _source to reduce
        // payload; the cursor we need lives in hit["sort"], not _source.
        let page_start = global_index;
        let page_end = global_index + PAGE_SIZE; // upper bound, may overshoot
        let page_entirely_before_range = range.is_some() && page_end <= range_start;
        let page_entirely_after_range = range.is_some() && page_start >= range_end;

        if page_entirely_after_range {
            break;
        }

        let mut body = json!({
            "query": { "term": { "taxon_id": taxon_id } },
            "size": PAGE_SIZE,
            "sort": [{ "uniprot_accession_number": "asc" }]
        });

        if page_entirely_before_range {
            // Suppress _source; we only need the sort cursor from these docs.
            body["_source"] = json!(false);
        } else if let Some(fields) = source_filter {
            body["_source"] = json!(fields);
        }

        if let Some(ref cursor) = search_after {
            body["search_after"] = json!([cursor]);
        }

        let response = client
            .search(SearchParts::Index(&["uniprot_entries"]))
            .body(body)
            .send()
            .await?;

        if !response.status_code().is_success() {
            return Err(GeneralError(response.text().await?));
        }

        let response_body: serde_json::Value = response.json().await?;
        let hits = match response_body["hits"]["hits"].as_array() {
            Some(h) => h,
            None => break,
        };

        let hit_count = hits.len();

        if !page_entirely_before_range {
            all_sources.reserve(hit_count.min(range_end.saturating_sub(range_start)));
            for (i, hit) in hits.iter().enumerate() {
                let doc_index = global_index + i;
                if doc_index >= range_start && doc_index < range_end {
                    all_sources.push(hit["_source"].clone());
                }
            }
        }

        global_index += hit_count;

        if hit_count < PAGE_SIZE {
            break;
        }

        if let Some(last_hit) = hits.last() {
            match last_hit["sort"][0].as_str() {
                Some(sort_val) => search_after = Some(sort_val.to_string()),
                None => break,
            }
        }
    }

    Ok(all_sources)
}

/// Retrieves all proteins from the database that belong to the given taxon ID.
/// Uses `search_after` pagination to handle taxa with more than 10,000 proteins.
pub async fn get_proteins_for_taxon(
    client: &OpenSearch,
    taxon_id: u32,
) -> Result<Vec<UniprotEntry>, DatabaseError> {
    Ok(fetch_taxon_sources(client, taxon_id, None, None)
        .await?
        .into_iter()
        .filter_map(|src| serde_json::from_value(src).ok())
        .collect())
}

/// Retrieves only the amino acid sequences for all UniProt entries associated with `taxon_id`.
/// Fetches only the `sequence` field from OpenSearch, reducing payload size compared to
/// `get_proteins_for_taxon` for callers that do not need the full `UniprotEntry`.
pub async fn get_protein_sequences_for_taxon(
    client: &OpenSearch,
    taxon_id: u32,
) -> Result<Vec<String>, DatabaseError> {
    #[derive(Deserialize)]
    struct SeqDoc { sequence: String }

    Ok(fetch_taxon_sources(client, taxon_id, Some(&["sequence"]), None)
        .await?
        .into_iter()
        .filter_map(|src| serde_json::from_value::<SeqDoc>(src).ok())
        .map(|d| d.sequence)
        .collect())
}

/// Retrieves the amino acid sequences for the half-open protein slice `[start, end)` belonging
/// to `taxon_id`, ordered by `uniprot_accession_number` (ascending).
///
/// Proteins are indexed by their position in the stable `uniprot_accession_number` sort order.
/// Uses `search_after` pagination, so the slice is not constrained by OpenSearch's default
/// 10,000-document `max_result_window`. An empty `Vec` is returned when `start >= end`.
pub async fn get_protein_sequences_for_taxon_range(
    client: &OpenSearch,
    taxon_id: u32,
    start: usize,
    end: usize,
) -> Result<Vec<String>, DatabaseError> {
    #[derive(Deserialize)]
    struct SeqDoc { sequence: String }

    Ok(fetch_taxon_sources(client, taxon_id, Some(&["sequence"]), Some((start, end)))
        .await?
        .into_iter()
        .filter_map(|src| serde_json::from_value::<SeqDoc>(src).ok())
        .map(|d| d.sequence)
        .collect())
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
    end: usize
) -> Result<Vec<String>, DatabaseError> {
    let body;

    // If filter is empty, use match_all query to count all documents
    if filter.is_empty() {
        body = json!({
            "query": {
                "match_all": {}
            }
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
            })
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
            }
        });
    }

    let response = client
        .search(SearchParts::Index(&["uniprot_entries"]))
        .from(start as i64)
        .size((end - start) as i64)
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
