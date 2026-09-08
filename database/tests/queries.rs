//! The OpenSearch queries this crate builds, asserted against a mock server.
//!
//! `Database::try_from_url` performs no I/O — it parses a URL and builds a transport — so pointing
//! it at a local mock is a complete substitute for a cluster, with no trait abstraction and no
//! change to production code.
//!
//! Each mock matches only a request of the expected shape, so `mock.assert()` failing means the
//! query was built differently, not that the server was never called. The query DSL travels in the
//! body; `from` and `size` are query parameters, and they are asserted where they actually appear.

use std::collections::HashSet;

use database::{
    Database, get_accessions, get_accessions_by_filter, get_accessions_count_by_filter, get_accessions_map
};
use httpmock::{Method::POST, MockServer};
use serde_json::json;

/// One OpenSearch `_source` document, in the shape `UniprotEntry` deserialises.
///
/// `version` and `taxon_id` arrive as strings and are converted during deserialisation, which is
/// why they are quoted here.
fn source(accession: &str, taxon_id: u32) -> serde_json::Value {
    json!({
        "uniprot_accession_number": accession,
        "version": "1",
        "taxon_id": taxon_id.to_string(),
        "type": "swissprot",
        "name": format!("Protein {accession}"),
        "sequence": "MKTAYIAKQR",
        "fa": "EC:1.1.1.1;GO:0009279"
    })
}

fn database(server: &MockServer) -> Database {
    Database::try_from_url(&server.base_url()).expect("a mock server URL should build a client")
}

/// An empty request set short-circuits before any request is made.
///
/// Worth its own test because the saving is invisible in the result: an empty vec comes back either
/// way, and only the mock's hit count distinguishes "asked for nothing" from "asked for everything".
#[tokio::test]
async fn an_empty_accession_set_makes_no_request() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).json_body(json!({ "docs": [] }));
        })
        .await;

    let database = database(&server);
    let result = get_accessions(database.get_conn(), &HashSet::new()).await.expect("an empty set should succeed");

    assert!(result.is_empty());
    mock.assert_hits_async(0).await;
}

#[tokio::test]
async fn accessions_are_fetched_by_mget_and_parsed() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_mget")
                .json_body_partial(r#"{ "docs": [ { "_id": "P00001" } ] }"#);
            then.status(200).json_body(json!({
                "docs": [ { "_source": source("P00001", 8501) } ]
            }));
        })
        .await;

    let database = database(&server);
    let accessions = HashSet::from(["P00001".to_string()]);
    let entries = get_accessions(database.get_conn(), &accessions).await.expect("the mocked response should parse");

    mock.assert_async().await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].uniprot_accession_number, "P00001");
    assert_eq!(entries[0].taxon_id, 8501);
    assert_eq!(entries[0].protein, "MKTAYIAKQR");
}

#[tokio::test]
async fn get_accessions_map_keys_entries_by_accession() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({
                "docs": [ { "_source": source("P00001", 8501) }, { "_source": source("P00003", 8502) } ]
            }));
        })
        .await;

    let database = database(&server);
    let accessions = HashSet::from(["P00001".to_string(), "P00003".to_string()]);
    let map = get_accessions_map(database.get_conn(), &accessions)
        .await
        .expect("the mocked response should parse");

    assert_eq!(map.len(), 2);
    assert_eq!(map["P00001"].taxon_id, 8501);
    assert_eq!(map["P00003"].taxon_id, 8502);
}

/// OpenSearch returns a `docs` entry for an id it does not hold, without a `_source`. Those are
/// dropped rather than failing the whole batch, so one unknown accession cannot lose the rest.
#[tokio::test]
async fn a_document_without_a_source_is_skipped() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({
                "docs": [ { "_id": "P99999", "found": false }, { "_source": source("P00001", 8501) } ]
            }));
        })
        .await;

    let database = database(&server);
    let accessions = HashSet::from(["P00001".to_string(), "P99999".to_string()]);
    let entries = get_accessions(database.get_conn(), &accessions).await.expect("a missing document is not an error");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].uniprot_accession_number, "P00001");
}

#[tokio::test]
async fn a_non_success_response_is_an_error() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(500).body("index is closed");
        })
        .await;

    let database = database(&server);
    let accessions = HashSet::from(["P00001".to_string()]);
    let error = get_accessions(database.get_conn(), &accessions).await.expect_err("a 500 must not be read as data");

    assert!(error.to_string().contains("index is closed"), "the response body should reach the caller: {error}");
}

/// An empty filter counts every document. `track_total_hits` is what makes the count exact rather
/// than capped at 10,000, so it is asserted rather than assumed.
#[tokio::test]
async fn an_empty_filter_counts_everything_with_match_all() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("size", "0")
                .json_body_partial(r#"{ "track_total_hits": true, "query": { "match_all": {} } }"#);
            then.status(200).json_body(json!({ "hits": { "total": { "value": 4321 } } }));
        })
        .await;

    let database = database(&server);
    let count = get_accessions_count_by_filter(database.get_conn(), String::new()).await.expect("the count parses");

    mock.assert_async().await;
    assert_eq!(count, 4321);
}

#[tokio::test]
async fn a_text_filter_matches_on_name_and_accession() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0").json_body_partial(
                r#"{ "track_total_hits": true, "query": { "bool": { "minimum_should_match": 1, "should": [
                   { "wildcard": { "name": { "value": "*croc*", "case_insensitive": true } } },
                   { "prefix": { "uniprot_accession_number": { "value": "croc", "case_insensitive": true } } }
                 ] } } }"#
            );
            then.status(200).json_body(json!({ "hits": { "total": { "value": 2 } } }));
        })
        .await;

    let database = database(&server);
    let count = get_accessions_count_by_filter(database.get_conn(), "croc".to_string()).await.expect("counts");

    mock.assert_async().await;
    assert_eq!(count, 2);
}

/// A filter that parses as an integer gains a third clause, so a taxon id can be searched for
/// directly. A filter that does not parse must not gain one — otherwise every text search would
/// carry a clause matching nothing.
#[tokio::test]
async fn a_numeric_filter_also_matches_the_taxon_id() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            // The whole `should` array is spelled out: a partial match compares arrays as a unit, and
            // the point of this test is that the numeric clause is *added* to the two text clauses
            // rather than replacing them.
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0").json_body_partial(
                r#"{ "track_total_hits": true, "query": { "bool": { "minimum_should_match": 1, "should": [
                   { "wildcard": { "name": { "value": "*8501*", "case_insensitive": true } } },
                   { "prefix": { "uniprot_accession_number": { "value": "8501", "case_insensitive": true } } },
                   { "match": { "taxon_id": { "query": 8501 } } }
                 ] } } }"#
            );
            then.status(200).json_body(json!({ "hits": { "total": { "value": 1 } } }));
        })
        .await;

    let database = database(&server);
    let count = get_accessions_count_by_filter(database.get_conn(), "8501".to_string()).await.expect("counts");

    mock.assert_async().await;
    assert_eq!(count, 1);
}

/// The caller passes a start and an end; OpenSearch takes an offset and a length. Getting that
/// subtraction wrong returns a page of the wrong size, which no assertion on the returned
/// accessions would notice as long as the mock returns a plausible list.
#[tokio::test]
async fn pagination_converts_start_and_end_into_from_and_size() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("from", "10")
                .query_param("size", "5");
            then.status(200).json_body(json!({
                "hits": { "hits": [
                    { "_source": { "uniprot_accession_number": "P00001" } },
                    { "_source": { "uniprot_accession_number": "P00003" } }
                ] }
            }));
        })
        .await;

    let database = database(&server);
    let accessions =
        get_accessions_by_filter(database.get_conn(), String::new(), 10, 15).await.expect("the page parses");

    mock.assert_async().await;
    assert_eq!(accessions, vec!["P00001", "P00003"]);
}

/// The corpus and the mocks have to agree about which proteins exist, or an `AppState` assembled
/// from both answers every query with nothing.
#[tokio::test]
async fn canned_documents_cover_the_corpus_accessions() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({
                "docs": fixtures::ACCESSIONS.iter().map(|a| json!({ "_source": source(a, 8501) })).collect::<Vec<_>>()
            }));
        })
        .await;

    let database = database(&server);
    let requested: HashSet<String> = fixtures::ACCESSIONS.iter().map(|a| a.to_string()).collect();
    let map = get_accessions_map(database.get_conn(), &requested).await.expect("the corpus batch parses");

    for accession in fixtures::ACCESSIONS {
        assert!(map.contains_key(accession), "{accession} is in the corpus but not in the mocked database");
    }
}

/// The listing side of a filter, which was reachable by no test.
///
/// `get_accessions_by_filter` builds its own query rather than sharing one with
/// `get_accessions_count_by_filter`, and the two have drifted: the count clause for a numeric
/// filter is a `match`, this one is a `term`. Both are asserted, so the difference is at least
/// visible — `/private_api/proteins` calls both, and a filter that counts one set and lists
/// another is the failure this pins.
#[tokio::test]
async fn a_numeric_filter_lists_by_a_term_clause() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("from", "0")
                .query_param("size", "2")
                .json_body_partial(
                    r#"{ "query": { "bool": { "minimum_should_match": 1, "should": [
                           { "wildcard": { "name": { "value": "*8501*", "case_insensitive": true } } },
                           { "prefix": { "uniprot_accession_number": { "value": "8501", "case_insensitive": true } } },
                           { "term": { "taxon_id": 8501 } }
                         ] } } }"#
                );
            then.status(200).json_body(json!({
                "hits": { "hits": [ { "_source": { "uniprot_accession_number": "P00001" } } ] }
            }));
        })
        .await;

    let database = database(&server);
    let found = get_accessions_by_filter(database.get_conn(), "8501".to_string(), 0, 2)
        .await
        .expect("the page parses");

    mock.assert_async().await;
    assert_eq!(found, vec!["P00001"]);
}

/// A filter that is not a number gains no taxon clause on the listing side either.
///
/// Without this, a change that always appended the clause would leave every text search carrying
/// one that matches nothing, and the count test alone would not notice.
#[tokio::test]
async fn a_text_filter_lists_without_a_taxon_clause() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").json_body_partial(
                r#"{ "query": { "bool": { "minimum_should_match": 1, "should": [
                       { "wildcard": { "name": { "value": "*croc*", "case_insensitive": true } } },
                       { "prefix": { "uniprot_accession_number": { "value": "croc", "case_insensitive": true } } }
                     ] } } }"#
            );
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let database = database(&server);
    let found = get_accessions_by_filter(database.get_conn(), "croc".to_string(), 0, 10).await.expect("parses");

    mock.assert_async().await;
    assert!(found.is_empty());
}

/// An `end` below `start` asks for a negative page, and must not underflow.
///
/// The handler rejects this ordering with a 400 before it reaches here, but the crate cannot assume
/// its caller does.
#[tokio::test]
async fn an_end_below_start_is_an_empty_page_not_an_underflow() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("from", "10")
                .query_param("size", "0");
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let database = database(&server);
    let accessions =
        get_accessions_by_filter(database.get_conn(), String::new(), 10, 0).await.expect("the page parses");

    mock.assert_async().await;
    assert!(accessions.is_empty());
}

/// A bound above `i64::MAX` saturates rather than wrapping to a negative one.
///
/// `from` and `size` travel as `i64`. `end` is a `usize` a caller sets, so on a 64-bit target it
/// reaches `usize::MAX` — and the handler's `end < start` check passes for it, since it is the
/// ordering that is checked and not the magnitude. A plain cast would send the cluster `-1`.
#[tokio::test]
async fn a_bound_above_i64_saturates_rather_than_going_negative() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                .query_param("from", "0")
                .query_param("size", i64::MAX.to_string());
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let database = database(&server);
    let accessions = get_accessions_by_filter(database.get_conn(), String::new(), 0, usize::MAX)
        .await
        .expect("the page parses");

    mock.assert_async().await;
    assert!(accessions.is_empty());
}
