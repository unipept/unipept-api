//! `/api/v2/taxonomy` — taxon information, optionally with a lineage and with descendants.
//!
//! Four optional parameters, and the widest behaviour surface of the datastore-only endpoints.
//! Root is a branch of its own.

use axum::http::StatusCode;

use crate::common::get_json;

/// Descendant ids, as they arrive. The endpoint orders them, so two responses may be compared
/// element by element.
fn ids(value: &serde_json::Value) -> Vec<u64> {
    value.as_array().expect("descendants").iter().map(|id| id.as_u64().expect("an id")).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_taxon_is_named_and_ranked() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 8501);
    assert_eq!(body[0]["taxon_name"], "Crocodylus niloticus");
    assert_eq!(body[0]["taxon_rank"], "species");
}

#[tokio::test(flavor = "multi_thread")]
async fn several_taxa_come_back_in_one_call() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8501&input[]=9503&input[]=7").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(3));
}

/// All four combinations of the two lineage flags, since only one of the four is the default.
#[tokio::test(flavor = "multi_thread")]
async fn extra_and_names_choose_how_much_lineage_comes_back() {
    let (_, plain) = get_json("/api/v2/taxonomy?input[]=8501").await;
    assert!(plain[0].get("genus_id").is_none());

    let (_, named_only) = get_json("/api/v2/taxonomy?input[]=8501&names=true").await;
    assert!(named_only[0].get("genus_id").is_none(), "names without extra adds nothing");

    let (_, extra_only) = get_json("/api/v2/taxonomy?input[]=8501&extra=true").await;
    assert_eq!(extra_only[0]["genus_id"], 8500);
    assert!(extra_only[0].get("genus_name").is_none());

    let (status, both) = get_json("/api/v2/taxonomy?input[]=8501&extra=true&names=true").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(both[0]["genus_name"], "Crocodylus");
}

/// `descendants` is off by default, so the field is absent rather than empty.
#[tokio::test(flavor = "multi_thread")]
async fn descendants_are_absent_unless_asked_for() {
    let (_, without) = get_json("/api/v2/taxonomy?input[]=8500").await;
    assert!(without[0].get("descendants").is_none());

    let (status, with) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true").await;
    assert_eq!(status, StatusCode::OK);
    assert!(with[0]["descendants"].is_array());
}

/// The ids come back ascending.
#[tokio::test(flavor = "multi_thread")]
async fn a_genus_finds_its_species() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true&descendants_ranks[]=species").await;

    assert_eq!(status, StatusCode::OK);

    assert_eq!(ids(&body[0]["descendants"]), vec![8501, 8502, 8503]);
}

/// `descendants_ranks` defaults to `["species"]`, so asking without it is asking for species.
#[tokio::test(flavor = "multi_thread")]
async fn the_default_descendant_rank_is_species() {
    let (_, defaulted) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true").await;
    let (_, explicit) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true&descendants_ranks[]=species").await;

    assert_eq!(ids(&defaulted[0]["descendants"]), ids(&explicit[0]["descendants"]));
}

/// The whole response is comparable, ids included.
#[tokio::test(flavor = "multi_thread")]
async fn the_same_request_answers_with_the_descendants_in_the_same_order() {
    let (_, first) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true").await;
    let (_, second) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true").await;

    assert_eq!(first, second);
}

/// Two ranks give one ascending list, not one ascending run per rank.
///
/// Each rank is read separately, so what is asserted is that the parts are ordered together after
/// they are collected rather than each on its own.
#[tokio::test(flavor = "multi_thread")]
async fn several_ranks_answer_with_one_ordered_list() {
    let (status, body) = get_json(
        "/api/v2/taxonomy?input[]=8493&descendants=true&descendants_ranks[]=species&descendants_ranks[]=genus"
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let found = ids(&body[0]["descendants"]);
    assert!(found.windows(2).all(|pair| pair[0] < pair[1]), "not ascending, and without repeats: {found:?}");

    // The same ids, whichever order the ranks are named in.
    let (_, reversed) = get_json(
        "/api/v2/taxonomy?input[]=8493&descendants=true&descendants_ranks[]=genus&descendants_ranks[]=species"
    )
    .await;

    assert_eq!(ids(&reversed[0]["descendants"]), found);
}

#[tokio::test(flavor = "multi_thread")]
async fn several_descendant_ranks_are_collected_together() {
    let (status, body) = get_json(
        "/api/v2/taxonomy?input[]=8493&descendants=true&descendants_ranks[]=genus&descendants_ranks[]=species"
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let found = body[0]["descendants"].as_array().expect("descendants");
    assert!(found.len() >= 4, "the genus and its three species: {found:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rank_nothing_sits_at_is_an_empty_list() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=8500&descendants=true&descendants_ranks[]=forma").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["descendants"].as_array().map(Vec::len), Some(0));
}

/// Root does not walk a lineage: it collects every taxon at the requested rank beneath each domain.
///
/// Root's own rank, `no rank`, names no lineage column, so this path starts at domain instead.
#[tokio::test(flavor = "multi_thread")]
async fn root_reports_every_taxon_at_the_requested_rank() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=1&descendants=true&descendants_ranks[]=species").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_id"], 1);
    assert_eq!(body[0]["taxon_rank"], "no rank");
    assert_eq!(body[0]["descendants"].as_array().map(Vec::len), Some(7), "every species in the corpus");
}

#[tokio::test(flavor = "multi_thread")]
async fn root_carries_an_empty_lineage_when_extra_is_asked_for() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=1&extra=true&names=true").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_name"], "root");
    assert!(body[0]["genus_id"].is_null(), "root sits above every rank");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_taxon_is_omitted_rather_than_failing() {
    let (status, body) = get_json("/api/v2/taxonomy?input[]=999999999&input[]=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(1));
}

/// A taxon whose own rank holds a space finds its descendants.
///
/// The taxon carries `species group` and the lineage column is `species_group`, so the endpoint has
/// to cross between the two spellings to read the column at all.
#[tokio::test(flavor = "multi_thread")]
async fn a_species_group_finds_its_descendants() {
    let path = format!(
        "/api/v2/taxonomy?input[]={}&descendants=true&descendants_ranks[]=species_subgroup",
        fixtures::taxa::MELANOGASTER_GROUP
    );
    let (status, body) = get_json(&path).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_rank"], "species group");
    assert_eq!(ids(&body[0]["descendants"]), vec![fixtures::taxa::MELANOGASTER_SUBGROUP as u64]);
}

/// The same crossing from the second multi-word rank, `species subgroup`.
///
/// A taxon is its own descendant at its own rank, so the subgroup answers with itself.
#[tokio::test(flavor = "multi_thread")]
async fn a_species_subgroup_is_read_at_its_own_rank() {
    let path = format!(
        "/api/v2/taxonomy?input[]={}&descendants=true&descendants_ranks[]=species_subgroup",
        fixtures::taxa::MELANOGASTER_SUBGROUP
    );
    let (status, body) = get_json(&path).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["taxon_rank"], "species subgroup");
    assert_eq!(ids(&body[0]["descendants"]), vec![fixtures::taxa::MELANOGASTER_SUBGROUP as u64]);
}

/// `descendants_ranks` reads a rank spelled either way.
///
/// The lineage columns are keyed on `species_group`; every response writes `species group`, so a
/// caller handing back a rank the API gave it is answered rather than refused.
#[tokio::test(flavor = "multi_thread")]
async fn a_descendant_rank_is_read_with_a_space_or_an_underscore() {
    let path = |rank: &str| {
        format!(
            "/api/v2/taxonomy?input[]={}&descendants=true&descendants_ranks[]={}",
            fixtures::taxa::MELANOGASTER_GROUP,
            rank.replace(' ', "%20")
        )
    };

    let (keyed_status, keyed) = get_json(&path("species_subgroup")).await;
    let (spaced_status, spaced) = get_json(&path("species subgroup")).await;

    assert_eq!(keyed_status, StatusCode::OK);
    assert_eq!(spaced_status, StatusCode::OK);
    assert_eq!(keyed, spaced);
    assert_eq!(ids(&keyed[0]["descendants"]), vec![fixtures::taxa::MELANOGASTER_SUBGROUP as u64]);
}

/// Only the separator is normalised: a rank argument names a column and is matched exactly.
#[tokio::test(flavor = "multi_thread")]
async fn a_descendant_rank_is_read_case_sensitively() {
    for rank in ["SPECIES_SUBGROUP", "Species Subgroup"] {
        let (dir, state) = crate::common::offline_state();
        let request = axum::http::Request::get(format!(
            "/api/v2/taxonomy?input[]={}&descendants=true&descendants_ranks[]={}",
            fixtures::taxa::MELANOGASTER_GROUP,
            rank.replace(' ', "%20")
        ))
        .body(axum::body::Body::empty())
        .unwrap();
        let (status, _) = crate::common::request_raw(state, request).await;
        drop(dir);

        assert_eq!(status, StatusCode::BAD_REQUEST, "{rank}");
    }
}
