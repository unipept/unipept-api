//! `/api/v2/pept2prot` — the one endpoint needing the index, the database and the datastore at once.
//!
//! It is last because it is the only test of the composition itself. A peptide search finds
//! proteins, the cluster resolves their accessions, and the datastore names their taxa — three
//! sources that have to agree about which proteins exist. Assembled from separate fixtures this
//! endpoint would answer with an empty list and every assertion about a status code would still
//! pass, which is the failure the shared corpus was built to prevent.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};
use fixtures::peptides::*;
use httpmock::{Method::POST, MockServer};
use serde_json::json;

use crate::{
    common::{request_raw, test_state},
    database::{get_against, source, taxon_of}
};

/// A cluster that answers for every accession the corpus declares.
async fn cluster_holding_the_corpus() -> MockServer {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({
                // The taxon comes from the corpus rather than a constant, so a row attributing
                // one species' protein to another cannot pass.
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
    server
}

/// The composition, end to end: search, resolve, name.
#[tokio::test(flavor = "multi_thread")]
async fn pept2prot_joins_the_index_the_cluster_and_the_datastore() {
    let server = cluster_holding_the_corpus().await;
    let (status, body) = get_against(&server, &format!("/api/v2/pept2prot?input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(!body.as_array().expect("rows").is_empty(), "the corpus and the cluster must agree on P00001");
    assert_eq!(body[0]["peptide"], UNIQUE);
    assert_eq!(body[0]["uniprot_id"], "P00001", "the index found it and the cluster resolved it");
    assert_eq!(body[0]["protein_name"], "Corpus protein");
    assert_eq!(body[0]["taxon_id"], 8501);
}

/// `extra` pulls the taxon's name out of the datastore and splits the annotations by prefix.
///
/// The three reference fields are not formatted alike: `ec_references` and `go_references` keep
/// their prefixes and `interpro_references` drops its, because that builder alone slices `k[4..]`
/// where the other two call `to_string`. Asserted as it is rather than as it looks like it should
/// be — a client parsing all three the same way gets the InterPro one wrong.
#[tokio::test(flavor = "multi_thread")]
async fn extra_names_the_taxon_and_separates_the_annotations() {
    let server = cluster_holding_the_corpus().await;
    let (status, body) = get_against(&server, &format!("/api/v2/pept2prot?input[]={UNIQUE}&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus", "named out of the datastore, not the cluster");
    assert_eq!(body[0]["ec_references"], "EC:1.1.1.1", "prefix kept");
    assert_eq!(body[0]["go_references"], "GO:0009279", "prefix kept");
    assert_eq!(body[0]["interpro_references"], "IPR016364", "prefix dropped, unlike the other two");
}

/// A peptide reaching two proteins gets a row for each, and each row carries that protein's own
/// taxon — P00001 belongs to *C. niloticus* and P00003 to *C. porosus*.
#[tokio::test(flavor = "multi_thread")]
async fn a_shared_peptide_returns_a_row_per_protein() {
    let server = cluster_holding_the_corpus().await;
    let (status, body) = get_against(&server, &format!("/api/v2/pept2prot?input[]={GENUS_SHARED}")).await;

    assert_eq!(status, StatusCode::OK);

    let mut rows: Vec<(&str, u64)> = body
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| (row["uniprot_id"].as_str().expect("an id"), row["taxon_id"].as_u64().expect("a taxon")))
        .collect();
    rows.sort_unstable();

    assert_eq!(rows, vec![("P00001", 8501), ("P00003", 8502)], "each row keeps its own taxon");
}

/// A protein the index knows and the cluster does not is left out rather than failing the request.
///
/// This is the drift the shared corpus exists to prevent, forced deliberately: the cluster is told
/// to answer for nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_protein_the_cluster_cannot_resolve_is_left_out() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/uniprot_entries/_mget");
            then.status(200).json_body(json!({ "docs": [] }));
        })
        .await;

    let (status, body) = get_against(&server, &format!("/api/v2/pept2prot?input[]={UNIQUE}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]), "an unresolvable protein is dropped, not a 500");
}

/// A peptide in no protein short-circuits before the cluster is asked anything.
#[tokio::test(flavor = "multi_thread")]
async fn an_absent_peptide_asks_the_cluster_nothing() {
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).json_body(json!({ "docs": [] }));
        })
        .await;

    let (status, body) = get_against(&server, &format!("/api/v2/pept2prot?input[]={ABSENT}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
    mock.assert_hits_async(0).await;
}

/// A cluster that fails is a 500, not a partial answer.
#[tokio::test(flavor = "multi_thread")]
async fn a_failing_cluster_fails_the_request() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(500).body("index is closed");
        })
        .await;

    let (_dir, state) = test_state(&server.base_url());
    let request = Request::get(format!("/api/v2/pept2prot?input[]={UNIQUE}")).body(Body::empty()).unwrap();
    let (status, _) = request_raw(state, request).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

/// `tryptic` can only narrow which proteins a peptide reaches, never widen it.
#[tokio::test(flavor = "multi_thread")]
async fn tryptic_results_are_a_subset_of_untryptic_ones() {
    let server = cluster_holding_the_corpus().await;

    let (unfiltered_status, all) = get_against(&server, &format!("/api/v2/pept2prot?input[]={COMMON}")).await;
    let (status, tryptic) = get_against(&server, &format!("/api/v2/pept2prot?input[]={COMMON}&tryptic=true")).await;
    assert_eq!(unfiltered_status, StatusCode::OK);
    assert_eq!(status, StatusCode::OK);

    let accessions = |value: &serde_json::Value| -> std::collections::BTreeSet<String> {
        value
            .as_array()
            .expect("rows")
            .iter()
            .map(|row| row["uniprot_id"].as_str().expect("an id").to_string())
            .collect()
    };
    let (all, tryptic) = (accessions(&all), accessions(&tryptic));

    assert!(!all.is_empty(), "the common peptide must reach something, or this proves nothing");
    assert!(tryptic.is_subset(&all), "tryptic hits absent from the unfiltered result: {:?}", &tryptic - &all);
}

/// A cutoff truncates the proteins searched and says so on every row it returns.
#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_every_row() {
    let server = cluster_holding_the_corpus().await;

    let (uncapped_status, uncapped) = get_against(&server, &format!("/api/v2/pept2prot?input[]={COMMON}")).await;
    let (status, capped) = get_against(&server, &format!("/api/v2/pept2prot?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(uncapped_status, StatusCode::OK);
    assert_eq!(status, StatusCode::OK);
    assert!(uncapped.as_array().expect("rows").iter().all(|row| row["cutoff_used"] == false));
    assert!(capped.as_array().expect("rows").iter().all(|row| row["cutoff_used"] == true));
    assert!(
        capped.as_array().expect("rows").len() < uncapped.as_array().expect("rows").len(),
        "a cutoff of two should return fewer proteins than none"
    );
}

/// `equate_il` reaches a second protein, which becomes a second row.
#[tokio::test(flavor = "multi_thread")]
async fn equate_il_reaches_a_second_protein() {
    let server = cluster_holding_the_corpus().await;

    let (apart_status, apart) =
        get_against(&server, &format!("/api/v2/pept2prot?input[]={IL_ISOLEUCINE}&equate_il=false")).await;
    let (status, together) =
        get_against(&server, &format!("/api/v2/pept2prot?input[]={IL_ISOLEUCINE}&equate_il=true")).await;

    assert_eq!(apart_status, StatusCode::OK);
    assert_eq!(status, StatusCode::OK);
    assert_eq!(apart.as_array().map(Vec::len), Some(1));
    assert_eq!(together.as_array().map(Vec::len), Some(2));
}

/// A repeated peptide carries all of its proteins to each position it occupies.
///
/// The cluster is asked for each accession once regardless: the lookup is keyed on accession and
/// built from the distinct peptides, which reach the same proteins.
#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_carries_all_of_its_proteins_to_each_position() {
    let server = cluster_holding_the_corpus().await;

    let (status, once) = get_against(&server, &format!("/api/v2/pept2prot?input[]={GENUS_SHARED}&extra=true")).await;
    assert_eq!(status, StatusCode::OK);
    let once = once.as_array().expect("a list");
    assert!(once.len() > 1, "the peptide should reach several proteins, or this asserts nothing");

    let (status, twice) =
        get_against(&server, &format!("/api/v2/pept2prot?input[]={GENUS_SHARED}&input[]={GENUS_SHARED}&extra=true"))
            .await;
    assert_eq!(status, StatusCode::OK);
    let twice = twice.as_array().expect("a list");

    assert_eq!(twice.len(), once.len() * 2, "every protein repeats with the peptide");
    assert_eq!(&twice[..once.len()], &once[..]);
    assert_eq!(&twice[once.len()..], &once[..], "the second occurrence answers to the byte like the first");
}
