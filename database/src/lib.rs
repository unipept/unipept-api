use std::{collections::HashMap};
use std::collections::HashSet;
pub use errors::DatabaseError;
use models::UniprotEntry;
use opensearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use opensearch::http::{Url};
use opensearch::{OpenSearch, SearchParts};
use serde_json::json;
use crate::DatabaseError::GeneralError;

mod errors;
mod models;

pub struct Database {
    client: OpenSearch
}

impl Database {
    pub fn try_from_url(url: &str) -> Result<Self, DatabaseError> {
        let url = Url::parse(url)?;
        let conn_pool = SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(conn_pool).disable_proxy().build()?;
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
