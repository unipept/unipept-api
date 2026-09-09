//! `/private_api/taxa2rank` — maps groups of taxa onto an ancestor rank.
//!
//! The only endpoint here whose `rank` parameter is required and validated, so it is also the one
//! that can answer with a 400.

use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE}
};
use serde_json::json;

use crate::common::{offline_state, post_json, request_raw};

/// Posts a body and returns the status with the response as text, for the rejection cases.
async fn post_raw(body: serde_json::Value) -> (StatusCode, String) {
    let (dir, state) = offline_state();
    let request = Request::post("/private_api/taxa2rank")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let answered = request_raw(state, request).await;
    drop(dir);
    answered
}

#[tokio::test(flavor = "multi_thread")]
async fn two_species_map_onto_their_shared_genus() {
    let (status, body) = post_json("/private_api/taxa2rank", json!({ "taxa": [[8501, 8502]], "rank": "genus" })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapped_taxa"], json!([[8500]]), "both reduce to Crocodylus, deduplicated");
}

/// Each inner list is one peptide's taxa and keeps its own position in the answer.
#[tokio::test(flavor = "multi_thread")]
async fn each_group_is_mapped_independently() {
    let (status, body) =
        post_json("/private_api/taxa2rank", json!({ "taxa": [[8501], [9503], [8501, 8502]], "rank": "genus" })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapped_taxa"], json!([[8500], [9499], [8500]]));
}

/// The same taxa reduce differently depending on the rank asked for, which is the whole point of
/// the endpoint and needs more than one rank to show.
#[tokio::test(flavor = "multi_thread")]
async fn the_rank_chooses_how_far_up_the_answer_sits() {
    for (rank, expected) in [("species", json!([[8501]])), ("genus", json!([[8500]])), ("family", json!([[8493]]))] {
        let (status, body) = post_json("/private_api/taxa2rank", json!({ "taxa": [[8501]], "rank": rank })).await;

        assert_eq!(status, StatusCode::OK, "{rank}");
        assert_eq!(body["mapped_taxa"], expected, "at rank {rank}");
    }
}

/// A taxon recording nothing at the requested rank contributes nothing rather than a null.
#[tokio::test(flavor = "multi_thread")]
async fn a_taxon_with_no_ancestor_at_that_rank_drops_out() {
    let (status, body) = post_json("/private_api/taxa2rank", json!({ "taxa": [[8501]], "rank": "class" })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapped_taxa"], json!([[]]), "Crocodylus niloticus records no class");
}

/// `rank` names a lineage column, so its case is significant.
///
/// The taxon filters match a rank without regard to case, because there the rank is text a user
/// typed against the name a response carries. This parameter is an identifier, and reads like
/// `descendants_ranks` on `/api/v2/taxonomy` rather than like a filter.
#[tokio::test(flavor = "multi_thread")]
async fn the_rank_case_is_significant() {
    let (status, body) = post_json("/private_api/taxa2rank", json!({ "taxa": [[8501]], "rank": "genus" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapped_taxa"], json!([[8500]]));

    for rank in ["GENUS", "Genus"] {
        let (status, _) = post_raw(json!({ "taxa": [[8501]], "rank": rank })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{rank}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_rank_is_rejected_with_the_rank_named() {
    let (status, body) = post_raw(json!({ "taxa": [[8501]], "rank": "not a rank" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Invalid rank"), "the message should name the problem: {body}");
    assert!(body.contains("not a rank"), "and quote what was asked for: {body}");
}

/// `rank` has no default, unlike every other parameter in this suite.
#[tokio::test(flavor = "multi_thread")]
async fn a_missing_rank_is_rejected() {
    let (status, _) = post_raw(json!({ "taxa": [[8501]] })).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread")]
async fn no_taxa_is_an_empty_answer() {
    let (status, body) = post_json("/private_api/taxa2rank", json!({ "taxa": [], "rank": "genus" })).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapped_taxa"], json!([]));
}

/// `rank` is read spelled either way, so a caller can hand back the name a response carries.
///
/// Only the separator is normalised: the name is matched with regard to case.
#[tokio::test(flavor = "multi_thread")]
async fn the_rank_is_read_with_a_space_or_an_underscore() {
    let taxa = json!([[fixtures::taxa::MELANOGASTER_SUBGROUP]]);

    let (keyed_status, keyed) =
        post_json("/private_api/taxa2rank", json!({ "taxa": taxa, "rank": "species_group" })).await;
    let (spaced_status, spaced) =
        post_json("/private_api/taxa2rank", json!({ "taxa": taxa, "rank": "species group" })).await;

    assert_eq!(keyed_status, StatusCode::OK);
    assert_eq!(spaced_status, StatusCode::OK);
    assert_eq!(keyed, spaced);
    assert_eq!(keyed["mapped_taxa"], json!([[fixtures::taxa::MELANOGASTER_GROUP]]));
}
