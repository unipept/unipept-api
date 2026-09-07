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
