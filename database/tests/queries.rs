//! The OpenSearch queries this crate builds, asserted against a mock server.
//!
//! `Database::try_from_url` performs no I/O — it parses a URL and builds a transport — so pointing
//! it at a local mock is a complete substitute for a cluster, with no trait abstraction and no
//! change to production code.
//!
//! Each mock matches only a request of the expected shape, so `mock.assert()` failing means the
//! query was built differently, not that the server was never called. The query DSL travels in the
//! body; `from` and `size` are query parameters, and they are asserted where they actually appear.

use database::{
    Database, get_accessions, get_accessions_by_filter, get_accessions_count_by_filter, get_accessions_map
};
use httpmock::{Method::POST, MockServer};
use serde_json::json;

/// The live protein count, which is what makes a last-page request a deep one.
const TOTAL: usize = 149_655_504;

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
    let result = get_accessions(database.get_conn(), &[]).await.expect("an empty set should succeed");

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
    let accessions = ["P00001".to_string()];
    let entries = get_accessions(database.get_conn(), &accessions).await.expect("the mocked response should parse");

    mock.assert_async().await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].uniprot_accession_number, "P00001");
    assert_eq!(entries[0].taxon_id, 8501);
    assert_eq!(entries[0].protein, "MKTAYIAKQR");
}

/// The ids reach OpenSearch in the order they were given, and `mget` answers a `docs` array in
/// that same order, so the caller's order is what comes back.
#[tokio::test]
async fn accessions_are_requested_in_the_order_they_are_given() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_mget")
                .json_body(json!({ "docs": [ { "_id": "P00003" }, { "_id": "P00001" } ] }));
            then.status(200).json_body(json!({
                "docs": [ { "_source": source("P00003", 8502) }, { "_source": source("P00001", 8501) } ]
            }));
        })
        .await;

    let database = database(&server);
    let accessions = ["P00003".to_string(), "P00001".to_string()];
    let entries = get_accessions(database.get_conn(), &accessions).await.expect("the mocked response should parse");

    mock.assert_async().await;
    assert_eq!(entries.iter().map(|entry| entry.uniprot_accession_number.as_str()).collect::<Vec<_>>(), vec![
        "P00003", "P00001"
    ]);
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
    let accessions = ["P00001".to_string(), "P00003".to_string()];
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
    let accessions = ["P00001".to_string(), "P99999".to_string()];
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
    let accessions = ["P00001".to_string()];
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
                   { "term": { "taxon_id": 8501 } }
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
    let requested: Vec<String> = fixtures::ACCESSIONS.iter().map(|a| a.to_string()).collect();
    let map = get_accessions_map(database.get_conn(), &requested).await.expect("the corpus batch parses");

    for accession in fixtures::ACCESSIONS {
        assert!(map.contains_key(accession), "{accession} is in the corpus but not in the mocked database");
    }
}

/// A numeric filter reaches the listing as a `term` clause on the taxon id.
///
/// `the_count_and_the_listing_select_the_same_set` holds that both sides send one query; this
/// pins the shape of that query on the listing side, so a change to it fails here rather than
/// only where the two are compared.
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

/// A window running past the last entry asks for what there is.
///
/// `end` is a `usize` a caller sets, so on a 64-bit target it reaches `usize::MAX` — the browser's
/// "All" option computes exactly that. It passes the result window, so it is clamped to the size of
/// the result set and reached from the other end, rather than being sent to the cluster as a `size`
/// that would saturate `i64`.
#[tokio::test]
async fn an_end_past_the_last_entry_is_clamped_to_it() {
    let server = MockServer::start_async().await;
    let count = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0");
            then.status(200).json_body(json!({ "hits": { "total": { "value": 3 } } }));
        })
        .await;
    let list = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("from", "0").query_param("size", "3");
            then.status(200).json_body(json!({ "hits": { "hits": [
                { "_source": { "uniprot_accession_number": "P00003" } },
                { "_source": { "uniprot_accession_number": "P00002" } },
                { "_source": { "uniprot_accession_number": "P00001" } }
            ] } }));
        })
        .await;

    let database = database(&server);
    let accessions = get_accessions_by_filter(database.get_conn(), String::new(), 0, usize::MAX)
        .await
        .expect("the page parses");

    count.assert_async().await;
    list.assert_async().await;
    assert_eq!(accessions, vec!["P00001", "P00002", "P00003"], "the reversed page is turned back around");
}

/// Counting and listing select the same set.
///
/// Deep paging makes the agreement load-bearing: the offset from the end is computed from the count
/// and applied to the listing, so two different sets give the wrong rows and no error. Nothing else
/// compares the two queries, which is why this does.
///
/// Asserted by matching one mock against both queries and letting the hit count say so.
#[tokio::test]
async fn the_count_and_the_listing_select_the_same_set() {
    let server = MockServer::start_async().await;
    let shared = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").json_body_partial(
                r#"{ "query": { "bool": { "minimum_should_match": 1, "should": [
                       { "wildcard": { "name": { "value": "*8501*", "case_insensitive": true } } },
                       { "prefix": { "uniprot_accession_number": { "value": "8501", "case_insensitive": true } } },
                       { "term": { "taxon_id": 8501 } }
                     ] } } }"#
            );
            then.status(200).json_body(json!({ "hits": { "total": { "value": 1 }, "hits": [] } }));
        })
        .await;

    let database = database(&server);
    get_accessions_count_by_filter(database.get_conn(), "8501".to_string()).await.expect("counts");
    get_accessions_by_filter(database.get_conn(), "8501".to_string(), 0, 10).await.expect("lists");

    shared.assert_hits_async(2).await;
}

/// The last page is reached by reversing the order, not by paging to it.
///
/// `from + size` cannot pass the result window, so the last page of a large set cannot be asked for
/// directly. Reversing a total order turns entry `total - 1` into entry `0`, which brings it inside
/// the window; the rows come back reversed and are turned around again.
///
/// 149,655,504 is the live protein count, and 5 per page is what the browser asks for, so this is
/// the request the browser's last-page button makes.
#[tokio::test]
async fn the_last_page_is_reached_from_the_other_end() {
    let server = MockServer::start_async().await;

    let count = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0");
            then.status(200).json_body(json!({ "hits": { "total": { "value": TOTAL } } }));
        })
        .await;
    let list = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/uniprot_entries/_search")
                // The window starts at the very end, and the order runs the other way.
                .query_param("from", "0")
                .query_param("size", "5")
                .json_body_partial(r#"{ "sort": [ { "uniprot_accession_number": { "order": "desc" } } ] }"#);
            then.status(200).json_body(json!({ "hits": { "hits": [
                { "_source": { "uniprot_accession_number": "Z00005" } },
                { "_source": { "uniprot_accession_number": "Z00004" } },
                { "_source": { "uniprot_accession_number": "Z00003" } },
                { "_source": { "uniprot_accession_number": "Z00002" } },
                { "_source": { "uniprot_accession_number": "Z00001" } }
            ] } }));
        })
        .await;

    let database = database(&server);
    let page = get_accessions_by_filter(database.get_conn(), String::new(), TOTAL - 5, TOTAL)
        .await
        .expect("the last page parses");

    count.assert_async().await;
    list.assert_async().await;
    assert_eq!(page, vec!["Z00001", "Z00002", "Z00003", "Z00004", "Z00005"], "the caller reads it forwards");
}

/// A page in the middle is reachable from neither end, and says so rather than being served wrong.
///
/// The mock covers the count only; the listing is asserted never to be asked.
#[tokio::test]
async fn a_page_in_the_middle_is_refused() {
    let server = MockServer::start_async().await;

    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0");
            then.status(200).json_body(json!({ "hits": { "total": { "value": TOTAL } } }));
        })
        .await;
    let list = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("from", "74827747");
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let database = database(&server);
    let error = get_accessions_by_filter(database.get_conn(), String::new(), TOTAL / 2, TOTAL / 2 + 5)
        .await
        .expect_err("the middle cannot be reached");

    assert!(matches!(error, database::DatabaseError::WindowUnreachable { .. }), "got: {error}");
    list.assert_hits_async(0).await;
}

/// A shallow page never asks for the count.
///
/// The extra query is what pays for reaching a deep page, and every page the browser opens with is
/// shallow. Asserted by hit count: the count mock must go untouched.
#[tokio::test]
async fn a_shallow_page_does_not_count_first() {
    let server = MockServer::start_async().await;
    let count = server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0");
            then.status(200).json_body(json!({ "hits": { "total": { "value": TOTAL } } }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "5");
            then.status(200).json_body(json!({ "hits": { "hits": [] } }));
        })
        .await;

    let database = database(&server);
    get_accessions_by_filter(database.get_conn(), String::new(), 0, 5).await.expect("the page parses");

    count.assert_hits_async(0).await;
}
