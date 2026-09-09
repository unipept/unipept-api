//! `/api/v2/pept2funct` — the EC, GO and InterPro annotations at once.
//!
//! It is the other three endpoints combined, so what matters is that it agrees with each of them
//! taken alone: nothing else checks the four have stayed in step.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

#[tokio::test(flavor = "multi_thread")]
async fn it_carries_all_three_annotation_kinds() {
    let (status, body) = get_json(&format!("/api/v2/pept2funct?input[]={UNIQUE}&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["ec"][0]["ec_number"], "1.1.1.1");
    assert_eq!(body[0]["go"][0]["go_term"], "GO:0009279");
    assert_eq!(body[0]["ipr"][0]["code"], "IPR016364");
}

#[tokio::test(flavor = "multi_thread")]
async fn it_agrees_with_the_three_endpoints_it_combines() {
    let (status, funct) = get_json(&format!("/api/v2/pept2funct?input[]={UNIQUE}&extra=true")).await;
    let (_, ec) = get_json(&format!("/api/v2/pept2ec?input[]={UNIQUE}&extra=true")).await;
    let (_, go) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}&extra=true")).await;
    let (_, ipr) = get_json(&format!("/api/v2/pept2interpro?input[]={UNIQUE}&extra=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(funct[0]["ec"], ec[0]["ec"]);
    assert_eq!(funct[0]["go"], go[0]["go"]);
    assert_eq!(funct[0]["ipr"], ipr[0]["ipr"]);
}

/// `domains` reshapes the GO and InterPro halves here too, and the agreement has to survive it.
#[tokio::test(flavor = "multi_thread")]
async fn it_agrees_with_them_under_domains_as_well() {
    let (status, funct) = get_json(&format!("/api/v2/pept2funct?input[]={UNIQUE}&extra=true&domains=true")).await;
    let (_, go) = get_json(&format!("/api/v2/pept2go?input[]={UNIQUE}&extra=true&domains=true")).await;
    let (_, ipr) = get_json(&format!("/api/v2/pept2interpro?input[]={UNIQUE}&extra=true&domains=true")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(funct[0]["go"], go[0]["go"]);
    assert_eq!(funct[0]["ipr"], ipr[0]["ipr"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cutoff_is_reported_on_the_result() {
    let (status, capped) = get_json(&format!("/api/v2/pept2funct?input[]={COMMON}&cutoff=2")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(capped[0]["cutoff_used"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_protein_with_no_annotations_still_answers() {
    let (status, body) = get_json(&format!("/api/v2/pept2funct?input[]={VALIDATION_SHARED}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["ec"].as_array().map(Vec::len), Some(0));
}

/// The terms come out most frequent first, ties broken on the identifier.
///
/// `AAGGK` states both halves of the rule. Its InterPro entries rank 2 before 1, which alphabetical
/// order alone would reverse; its two GO terms both count 2, so only the tiebreak separates them.
#[tokio::test(flavor = "multi_thread")]
async fn terms_come_back_most_frequent_first() {
    let (status, body) = get_json(&format!("/api/v2/pept2funct?input[]={COMMON}&equate_il=true")).await;

    assert_eq!(status, StatusCode::OK);

    assert_eq!(body[0]["ec"][0]["ec_number"], "1.1.1.1", "count 2");
    assert_eq!(body[0]["ec"][1]["ec_number"], "2.7.11.1", "count 1");

    assert_eq!(body[0]["ipr"][0]["code"], "IPR016364", "count 2, and the later code");
    assert_eq!(body[0]["ipr"][1]["code"], "IPR008816", "count 1");

    assert_eq!(body[0]["go"][0]["go_term"], "GO:0005515", "tied at 2, so the lower term first");
    assert_eq!(body[0]["go"][1]["go_term"], "GO:0009279");
}

/// Two `HashMap`s in one thread hash with different seeds, so the two calls here build their
/// answers separately rather than reading one map twice.
#[tokio::test(flavor = "multi_thread")]
async fn the_same_request_answers_identically() {
    let query = format!("/api/v2/pept2funct?input[]={COMMON}&equate_il=true&extra=true&domains=true");

    let (status, first) = get_json(&query).await;
    let (_, again) = get_json(&query).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(first, again);
}

/// A repeated peptide is searched once and answered at each position it occupies.
///
/// `pept2funct` reads three stores off one aggregation, so it is where searching twice cost most of
/// the annotation endpoints. Each row is compared against the row the peptide gets alone.
#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_answers_at_each_position() {
    let (status, once) = get_json(&format!("/api/v2/pept2funct?input[]={UNIQUE}&extra=true&domains=true")).await;
    assert_eq!(status, StatusCode::OK);

    let repeated = format!("input[]={UNIQUE}&input[]={GENUS_SHARED}&input[]={UNIQUE}&extra=true&domains=true");
    let (status, body) = get_json(&format!("/api/v2/pept2funct?{repeated}")).await;
    assert_eq!(status, StatusCode::OK);

    let rows = body.as_array().expect("a list");
    assert_eq!(rows.len(), 3, "one row per position: {body}");
    assert_eq!(rows[0], once[0]);
    assert_eq!(rows[2], once[0], "the second occurrence answers to the byte like the first");
    assert_eq!(rows[1]["peptide"], GENUS_SHARED);
}
