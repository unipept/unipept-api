//! `/private_api/proteins…` — the controllers backed by OpenSearch.
//!
//! Nothing here talks to a cluster. `Database::try_from_url` performs no I/O, so the state is
//! pointed at an `httpmock` server and every response is canned — keyed to `fixtures::ACCESSIONS`,
//! the same proteins the index holds, so a composed state answers with something rather than
//! plausibly with nothing.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use httpmock::MockServer;
use serde_json::Value;

use crate::common::{request_json, test_state};

/// One `_source` document in the shape `UniprotEntry` deserialises; the numbers arrive as strings.
pub fn source(accession: &str, taxon_id: u32, name: &str, fa: &str) -> Value {
    serde_json::json!({
        "uniprot_accession_number": accession,
        "version": "1",
        "taxon_id": taxon_id.to_string(),
        "type": "swissprot",
        "name": name,
        "sequence": "MKTAYIAKQRAWDIQNGK",
        "fa": fa
    })
}

/// Runs one GET against a state whose database is `server`.
pub async fn get_against(server: &MockServer, path: &str) -> (StatusCode, Value) {
    let (dir, state) = test_state(&server.base_url());
    let answered = request_json(state, Request::get(path).body(Body::empty()).unwrap()).await;
    drop(dir);
    answered
}

/// Runs one JSON POST against a state whose database is `server`.
pub async fn post_against(server: &MockServer, path: &str, body: Value) -> (StatusCode, Value) {
    let (dir, state) = test_state(&server.base_url());
    let request = Request::post(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let answered = request_json(state, request).await;
    drop(dir);
    answered
}

/// The taxon the corpus gives an accession.
///
/// Mocks that invent a taxon assert over the corpus rather than against it: a row attributing one
/// species' protein to another would look correct.
pub fn taxon_of(accession: &str) -> u32 {
    fixtures::PROTEINS_TSV
        .lines()
        .filter(|line| !line.trim().is_empty())
        .find_map(|line| {
            let mut fields = line.split('\t');
            (fields.next() == Some(accession)).then(|| fields.next().expect("a taxon column"))
        })
        .unwrap_or_else(|| panic!("{accession} is not in the corpus"))
        .parse()
        .expect("the taxon column is numeric")
}

/// The mocks answer on the index the fixture's `.version` names, which is the one the state
/// queries. Spelled out in `common` so a mock reads as a path; held to the fixture here.
#[test]
fn the_mocks_answer_on_the_index_the_fixture_names() {
    let index = database::index_name(fixtures::VERSION).expect("the fixture version names an index");

    assert_eq!(crate::common::MGET, format!("/{index}/_mget"));
    assert_eq!(crate::common::SEARCH, format!("/{index}/_search"));
}
