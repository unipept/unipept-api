//! What is true of every route rather than of any one controller.

use axum::http::StatusCode;
use fixtures::peptides::*;
use httpmock::{Method::POST, MockServer};
use serde_json::{Value, json};

use crate::{
    common::{get_json, post_json},
    database::{get_against, post_against, source, taxon_of}
};

/// Sorts every list in a response before it is compared: several controllers build a list by
/// iterating a `HashMap`, so element order differs between two identical requests.
fn canonical(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            let mut items: Vec<Value> = items.into_iter().map(canonical).collect();
            items.sort_by_key(Value::to_string);
            Value::Array(items)
        }
        Value::Object(fields) => Value::Object(fields.into_iter().map(|(key, item)| (key, canonical(item))).collect()),
        other => other
    }
}

/// Asserts that a query string and the JSON body meaning the same thing answer the same way.
fn assert_get_and_post_agree(query: &str, from_get: (StatusCode, Value), from_post: (StatusCode, Value)) {
    let (get_status, get_body) = from_get;
    let (post_status, post_body) = from_post;

    assert_eq!(get_status, StatusCode::OK, "GET {query}: {get_body}");
    assert_eq!(post_status, StatusCode::OK, "POST {query}: {post_body}");
    assert_ne!(get_body, json!([]), "{query} answered with an empty list, which proves nothing");
    assert_ne!(get_body, Value::Null, "{query} answered with null, which proves nothing");
    assert_eq!(canonical(get_body), canonical(post_body), "{query}");
}

/// The path a query string is sent to.
fn path_of(query: &str) -> &str {
    query.split('?').next().expect("a path")
}

/// A query string arrives as text and a JSON body carries real types, so each endpoint's own
/// `Parameters` deserialises by two different paths.
///
/// Not here: the cluster-backed routes (`get_and_post_answer_alike_against_a_cluster`), the two
/// `taxa2tree` routes, whose methods take different parameters
/// (`taxa2tree::counts_answer_like_the_repeats_they_stand_for`), and `/`, which is GET only.
#[tokio::test(flavor = "multi_thread")]
async fn get_and_post_answer_alike() {
    let peptides = format!("input[]={UNIQUE}&input[]={GENUS_SHARED}");
    let input = json!([UNIQUE, GENUS_SHARED]);

    let cases: Vec<(String, Value)> = vec![
        (
            format!("/api/v2/pept2ec?{peptides}&equate_il=true&extra=true&cutoff=100"),
            json!({ "input": input, "equate_il": true, "extra": true, "cutoff": 100 })
        ),
        (
            format!("/api/v2/pept2funct?{peptides}&equate_il=true&extra=true&domains=true&cutoff=100"),
            json!({ "input": input, "equate_il": true, "extra": true, "domains": true, "cutoff": 100 })
        ),
        (
            format!("/api/v2/pept2go?{peptides}&equate_il=true&extra=true&domains=true&cutoff=100"),
            json!({ "input": input, "equate_il": true, "extra": true, "domains": true, "cutoff": 100 })
        ),
        (
            format!("/api/v2/pept2interpro?{peptides}&equate_il=true&extra=true&domains=true&cutoff=100"),
            json!({ "input": input, "equate_il": true, "extra": true, "domains": true, "cutoff": 100 })
        ),
        (
            format!("/api/v2/pept2lca?{peptides}&equate_il=true&extra=true&names=true&validate_taxa=true&cutoff=100"),
            json!({
                "input": input,
                "equate_il": true,
                "extra": true,
                "names": true,
                "validate_taxa": true,
                "cutoff": 100
            })
        ),
        (
            format!("/api/v2/pept2taxa?{peptides}&equate_il=true&extra=true&names=true&tryptic=false&compact=true"),
            json!({
                "input": input,
                "equate_il": true,
                "extra": true,
                "names": true,
                "tryptic": false,
                "compact": true
            })
        ),
        (
            format!(
                "/api/v2/peptinfo?{peptides}&equate_il=true&extra=true&domains=true&names=true&validate_taxa=true&\
                 cutoff=100"
            ),
            json!({
                "input": input,
                "equate_il": true,
                "extra": true,
                "domains": true,
                "names": true,
                "validate_taxa": true,
                "cutoff": 100
            })
        ),
        (
            "/api/v2/taxa2lca?input[]=8501&input[]=8502&extra=true&names=true&validate_taxa=true".to_string(),
            json!({ "input": [8501, 8502], "extra": true, "names": true, "validate_taxa": true })
        ),
        (
            "/api/v2/taxonomy?input[]=8501&input[]=8502&extra=true&names=true&descendants=true&\
             descendants_ranks[]=species"
                .to_string(),
            json!({
                "input": [8501, 8502],
                "extra": true,
                "names": true,
                "descendants": true,
                "descendants_ranks": ["species"]
            })
        ),
        ("/datasets/sampledata".to_string(), json!({})),
        // `filter` is an externally tagged enum: a map in the body, a bracketed key in the query.
        (
            format!(
                "/mpa/pept2data?peptides[]={UNIQUE}&peptides[]={GENUS_SHARED}&equate_il=true&tryptic=false&\
                 cutoff=100&report_taxa=true&validate_taxa=true&filter[taxa][]=8501&filter[taxa][]=8502"
            ),
            json!({
                "peptides": input,
                "equate_il": true,
                "tryptic": false,
                "cutoff": 100,
                "report_taxa": true,
                "validate_taxa": true,
                "filter": { "taxa": [8501, 8502] }
            })
        ),
        (
            "/private_api/ecnumbers?ecnumbers[]=1.1.1.1&ecnumbers[]=2.7.11.1".to_string(),
            json!({ "ecnumbers": ["1.1.1.1", "2.7.11.1"] })
        ),
        (
            "/private_api/goterms?goterms[]=GO:0009279&goterms[]=GO:0005515".to_string(),
            json!({ "goterms": ["GO:0009279", "GO:0005515"] })
        ),
        ("/private_api/interpros?interpros[]=IPR016364".to_string(), json!({ "interpros": ["IPR016364"] })),
        ("/private_api/metadata".to_string(), json!({})),
        (
            "/private_api/proteomes?proteomes[]=UP000000001&proteomes[]=UP000000002".to_string(),
            json!({ "proteomes": ["UP000000001", "UP000000002"] })
        ),
        ("/private_api/proteomes/count?filter=UP".to_string(), json!({ "filter": "UP" })),
        (
            "/private_api/proteomes/filter?filter=UP&start=0&end=3&sort_by=name&sort_descending=true".to_string(),
            json!({ "filter": "UP", "start": 0, "end": 3, "sort_by": "name", "sort_descending": true })
        ),
        ("/private_api/taxa?taxids[]=8501&taxids[]=8502".to_string(), json!({ "taxids": [8501, 8502] })),
        ("/private_api/taxa/count?filter=Crocodylus".to_string(), json!({ "filter": "Crocodylus" })),
        (
            "/private_api/taxa/filter?filter=Crocodylus&start=0&end=3&sort_by=name&sort_descending=true".to_string(),
            json!({ "filter": "Crocodylus", "start": 0, "end": 3, "sort_by": "name", "sort_descending": true })
        ),
        // A list of lists, which a query string can only spell with an index.
        (
            "/private_api/taxa2rank?taxa[0][]=8501&taxa[0][]=8502&taxa[1][]=9503&rank=genus".to_string(),
            json!({ "taxa": [[8501, 8502], [9503]], "rank": "genus" })
        ),
    ];

    for (query, body) in cases {
        let from_get = get_json(&query).await;
        let from_post = post_json(path_of(&query), body).await;
        assert_get_and_post_agree(&query, from_get, from_post);
    }
}

/// The same assertion for the routes that ask OpenSearch something.
#[tokio::test(flavor = "multi_thread")]
async fn get_and_post_answer_alike_against_a_cluster() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({
                "docs": fixtures::ACCESSIONS
                    .iter()
                    .map(|accession| json!({
                        "_source": source(
                            accession,
                            taxon_of(accession),
                            "Corpus protein",
                            "EC:1.1.1.1;GO:0009279;IPR:IPR016364"
                        )
                    }))
                    .collect::<Vec<_>>()
            }));
        })
        .await;
    // Both search mocks match on the filter, so a request that lost it reaches no mock at all.
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("size", "0").body_contains("8501");
            then.status(200).json_body(json!({ "hits": { "total": { "value": 2 } } }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_search").query_param("from", "0").body_contains("8501");
            then.status(200).json_body(json!({ "hits": { "hits": [
                { "_source": { "uniprot_accession_number": "P00001" } },
                { "_source": { "uniprot_accession_number": "P00003" } }
            ] } }));
        })
        .await;

    let cases: Vec<(String, Value)> = vec![
        (
            format!("/api/v2/pept2prot?input[]={UNIQUE}&equate_il=true&extra=true&tryptic=false&cutoff=100"),
            json!({ "input": [UNIQUE], "equate_il": true, "extra": true, "tryptic": false, "cutoff": 100 })
        ),
        (
            "/api/v2/protinfo?input[]=P00001&extra=true&domains=true&names=true".to_string(),
            json!({ "input": ["P00001"], "extra": true, "domains": true, "names": true })
        ),
        (
            "/private_api/proteins?accessions[]=P00001&accessions[]=P00003".to_string(),
            json!({ "accessions": ["P00001", "P00003"] })
        ),
        ("/private_api/proteins/count?filter=8501".to_string(), json!({ "filter": "8501" })),
        (
            "/private_api/proteins/filter?filter=8501&start=0&end=2".to_string(),
            json!({ "filter": "8501", "start": 0, "end": 2 })
        ),
    ];

    for (query, body) in cases {
        let from_get = get_against(&server, &query).await;
        let from_post = post_against(&server, path_of(&query), body).await;
        assert_get_and_post_agree(&query, from_get, from_post);
    }
}

/// The same macro registers a `.json` suffix beside every path.
#[tokio::test(flavor = "multi_thread")]
async fn the_json_suffix_reaches_the_same_handler() {
    for (plain, suffixed) in [
        ("/private_api/metadata", "/private_api/metadata.json"),
        ("/api/v2/taxonomy?input[]=8501", "/api/v2/taxonomy.json?input[]=8501"),
        ("/datasets/sampledata", "/datasets/sampledata.json")
    ] {
        let (plain_status, from_plain) = get_json(plain).await;
        let (suffixed_status, from_suffixed) = get_json(suffixed).await;

        assert_eq!(plain_status, suffixed_status, "{plain}");
        assert_eq!(from_plain, from_suffixed, "{plain}");
    }
}

/// `/api/v1` and `/api/v2` mount the same router, which `create_api_routes` documents as
/// deliberate. Asserted so that separating them becomes a deliberate change rather than a
/// silent one.
#[tokio::test(flavor = "multi_thread")]
async fn the_two_api_versions_answer_identically() {
    let (v1_status, v1) = get_json("/api/v1/taxonomy?input[]=8501").await;
    let (v2_status, v2) = get_json("/api/v2/taxonomy?input[]=8501").await;

    assert_eq!(v1_status, v2_status);
    assert_eq!(v1, v2, "v1 is an alias for v2; see the comment on create_api_routes");
}
