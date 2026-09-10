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

/// The query that selects the entries a filter matches.
///
/// Counting and listing must select the same set. They were built separately before and had
/// drifted: a numeric filter was a `match` clause on the counting side and a `term` on the listing
/// side. That mattered little while the two were only compared by eye, and matters a great deal
/// now — a deep page is reached by counting first and paging in from the other end, and that
/// arithmetic is sound only if both answers describe one set.
///
/// `term` is the surviving spelling. `taxon_id` is mapped `integer`, so a `match` on it resolves to
/// the same term query; keeping the exact one says so.
fn entry_query(filter: &str) -> serde_json::Value {
    if filter.is_empty() {
        return json!({ "match_all": {} });
    }

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
        // Uniprot accession number starts with filter
        json!({
            "prefix": {
                "uniprot_accession_number": {
                    "value": filter,
                    "case_insensitive": true
                }
            }
        }),
    ];

    // A filter that parses as a number matches a taxon id as well. One that does not is no taxon
    // id, and the clause is left out rather than being matched against nothing.
    if let Ok(taxon_id) = filter.parse::<u32>() {
        should_conditions.push(json!({
            "term": {
                "taxon_id": taxon_id
            }
        }));
    }

    json!({
        "bool": {
            "should": should_conditions,
            "minimum_should_match": 1
        }
    })
}

/// Counts the UniProt entries a filter matches.
///
/// An entry counts when its name contains the filter, its accession starts with it, or — where the
/// filter is a number — its taxon id equals it. `entry_query` is what says so, and the listing
/// reads the same one.
///
/// `track_total_hits` is what makes this exact rather than capped at 10,000, which the deep-paging
/// arithmetic in `get_accessions_by_filter` depends on.
pub async fn get_accessions_count_by_filter(client: &OpenSearch, filter: String) -> Result<u32, DatabaseError> {
    let body = json!({
        "query": entry_query(&filter),
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

/// One window of the listing, as the cluster takes it.
///
/// `from + size` must stay inside `MAX_RESULT_WINDOW`; the caller is what guarantees that.
async fn search_window(
    client: &OpenSearch,
    filter: &str,
    from: usize,
    size: usize,
    sort_by: ProteinSortField,
    descending: bool
) -> Result<Vec<String>, DatabaseError> {
    let body = json!({
        "query": entry_query(filter),
        "sort": sort_clause(sort_by, descending)
    });

    let response = client
        .search(SearchParts::Index(&["uniprot_entries"]))
        .from(as_window_bound(from))
        .size(as_window_bound(size))
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

/// Gets UniProt accession IDs from the database that match the given filter criteria
///
/// # Arguments
/// * `client` - Database connection handle
/// * `filter` - String to filter entries by. If empty, returns unfiltered results
/// * `start` - Starting index for pagination
/// * `end` - Ending index for pagination
/// * `sort_by` - The field to order the listing by
/// * `sort_descending` - Whether that order runs the other way
///
/// # Returns
/// * Vector of UniProt accession IDs that match the filter criteria
/// * `DatabaseError::WindowUnreachable` if the page lies in the middle the cluster cannot reach
/// * `DatabaseError` if the database operation fails
///
/// # Reaching a deep page
///
/// OpenSearch refuses a search whose `from + size` passes `MAX_RESULT_WINDOW`, so the last page of
/// 149 million entries cannot be asked for directly. It can be asked for from the other end: the
/// order is total, so reversing it turns entry `total - 1` into entry `0`, and the last page
/// becomes the first. The rows come back reversed and are turned around again.
///
/// That reaches the first `MAX_RESULT_WINDOW` entries and the last `MAX_RESULT_WINDOW` of them.
/// A page between the two is refused: no ordering brings it inside the window from either side.
/// The table this serves offers first, previous, next and last, and no way to jump to a page — so
/// the middle is not somewhere a client can land in one step, only somewhere it can walk to.
pub async fn get_accessions_by_filter(
    client: &OpenSearch,
    filter: String,
    start: usize,
    end: usize,
    sort_by: ProteinSortField,
    sort_descending: bool
) -> Result<Vec<String>, DatabaseError> {
    // Inside the window, so the cluster answers it as asked. `end` is `from + size`, which is the
    // bound OpenSearch applies.
    if end <= MAX_RESULT_WINDOW {
        return search_window(client, &filter, start, end.saturating_sub(start), sort_by, sort_descending).await;
    }

    // Past it, so how far the page sits from the end decides whether it can be reached at all, and
    // that needs the size of the result set. One extra count, on a page a client reaches by asking
    // for it rather than by walking there.
    let total = get_accessions_count_by_filter(client, filter.clone()).await? as usize;

    // A window running past the last entry asks for what there is, as it does inside the window.
    let end = end.min(total);
    if start >= end {
        return Ok(Vec::new());
    }

    // Reversed, the window starts `total - end` in and is the same size, so `from + size` becomes
    // `total - start`. That is what has to fit.
    if total - start > MAX_RESULT_WINDOW {
        return Err(DatabaseError::WindowUnreachable { start, end, total, window: MAX_RESULT_WINDOW });
    }

    let mut page = search_window(client, &filter, total - end, end - start, sort_by, !sort_descending).await?;

    // Read back into the order the caller asked for.
    page.reverse();

    Ok(page)
}
