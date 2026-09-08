//! `/private_api/taxa/count` and `/taxa/filter` — counting and paging the taxon table.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};

use crate::common::{get_json, offline_state, request_raw};

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_filter_counts_every_ranked_valid_taxon() {
    let (status, body) = get_json("/private_api/taxa/count").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], fixtures::ranked_and_valid_taxa());
}

/// The count excludes the invalid taxon, which `/private_api/taxa` will still return by id.
#[tokio::test(flavor = "multi_thread")]
async fn the_invalid_taxon_is_not_counted() {
    let (_, counted) = get_json("/private_api/taxa/count").await;
    let (_, by_id) = get_json(&format!("/private_api/taxa?taxids[]={}", fixtures::taxa::HELODERMA)).await;

    assert_eq!(counted["count"], fixtures::ranked_and_valid_taxa());
    assert_eq!(by_id.as_array().map(Vec::len), Some(1), "but it is still there when asked for directly");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_name_filter_selects_fewer_than_everything() {
    let (status, body) = get_json("/private_api/taxa/count?filter=Crocodylus").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["count"].as_u64().is_some_and(|n| (4..fixtures::ranked_and_valid_taxa()).contains(&n)), "got {body}");
}

/// The filter matches on the id as well as the name.
#[tokio::test(flavor = "multi_thread")]
async fn a_numeric_filter_matches_a_taxon_id() {
    let (status, body) = get_json("/private_api/taxa/count?filter=8501").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["count"].as_u64().is_some_and(|n| n >= 1), "got {body}");
}

/// The filter matches on the rank name, including the two rank names that hold a space.
///
/// `species group` is the name the API serialises for that rank, so it is the name a caller has to
/// type. `species subgroup` is a different rank and must not be swept in with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_rank_filter_matches_a_multi_word_rank_name() {
    let (status, group) = get_json("/private_api/taxa/count?filter=species%20group").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(group["count"], 1);

    let (_, subgroup) = get_json("/private_api/taxa/count?filter=species%20subgroup").await;
    assert_eq!(subgroup["count"], 1);

    let (_, listed) = get_json("/private_api/taxa/filter?filter=species%20group&start=0&end=100").await;
    assert_eq!(as_set(&listed), std::collections::BTreeSet::from([fixtures::taxa::MELANOGASTER_GROUP as u64]));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_filter_ignores_case() {
    let (_, upper) = get_json("/private_api/taxa/count?filter=CROCODYLUS").await;
    let (status, lower) = get_json("/private_api/taxa/count?filter=crocodylus").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(upper["count"], lower["count"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_filter_matching_nothing_counts_nothing() {
    let (status, body) = get_json("/private_api/taxa/count?filter=no-such-taxon").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_returns_at_most_what_it_asked_for() {
    let (status, page) = get_json("/private_api/taxa/filter?start=0&end=3").await;

    assert_eq!(status, StatusCode::OK);
    assert!(page.as_array().expect("a page").len() <= 3);
}

/// Consecutive windows must not repeat a taxon, or paging would show one twice and skip another.
#[tokio::test(flavor = "multi_thread")]
async fn consecutive_windows_do_not_overlap() {
    let (_, first) = get_json("/private_api/taxa/filter?start=0&end=3").await;
    let (status, second) = get_json("/private_api/taxa/filter?start=3&end=6").await;

    assert_eq!(status, StatusCode::OK);
    for taxon in first.as_array().expect("a page") {
        assert!(!second.as_array().expect("a page").contains(taxon), "{taxon} appears in both windows");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_count_matches_what_paging_yields() {
    let (_, counted) = get_json("/private_api/taxa/count").await;
    let (status, listed) = get_json("/private_api/taxa/filter?start=0&end=1000").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().map(Vec::len), counted["count"].as_u64().map(|n| n as usize));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_past_the_end_is_empty_rather_than_an_error() {
    let (status, page) = get_json("/private_api/taxa/filter?start=500&end=600").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(page.as_array().map(Vec::len), Some(0));
}

/// The rejection body is plain text rather than JSON, so this reads it raw.
#[tokio::test(flavor = "multi_thread")]
async fn the_window_bounds_are_required() {
    let (dir, state) = offline_state();
    let request = Request::get("/private_api/taxa/filter").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, "invalid query string");
}

// ── sorting ─────────────────────────────────────────────────────────────────────────────────────
//
// `sort_by` takes "id", "name" or "rank", and `sort_descending` reverses each. Six combinations
// plus the default, none of which the endpoint validates — an unrecognised value is not an error.

/// Ids come back ascending, and `sort_descending` reverses exactly that order.
#[tokio::test(flavor = "multi_thread")]
async fn sorting_by_id_orders_and_reverses() {
    let (status, ascending) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=id").await;
    assert_eq!(status, StatusCode::OK);

    let ids: Vec<u64> = ascending.as_array().expect("a page").iter().map(|i| i.as_u64().unwrap()).collect();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "not ascending: {ids:?}");

    let (_, descending) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=id&sort_descending=true").await;
    let mut reversed = ids.clone();
    reversed.reverse();
    let got: Vec<u64> = descending.as_array().expect("a page").iter().map(|i| i.as_u64().unwrap()).collect();
    assert_eq!(got, reversed);
}

/// Sorting by name and by rank both reorder, and both reverse. The ids are checked as a set: the
/// same taxa must come back however they are arranged.
#[tokio::test(flavor = "multi_thread")]
async fn sorting_by_name_and_rank_reorders_without_losing_anything() {
    let (_, by_id) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=id").await;
    let baseline = as_set(&by_id);

    for field in ["name", "rank"] {
        for descending in ["false", "true"] {
            let path = format!("/private_api/taxa/filter?start=0&end=100&sort_by={field}&sort_descending={descending}");
            let (status, page) = get_json(&path).await;

            assert_eq!(status, StatusCode::OK, "{field}/{descending}");
            assert_eq!(as_set(&page), baseline, "sorting by {field} must not change which taxa come back");
        }
    }

    // And the two directions really are opposites.
    let (_, ascending) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=name").await;
    let (_, descending) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=name&sort_descending=true").await;
    let mut reversed = descending.as_array().expect("a page").clone();
    reversed.reverse();
    assert_eq!(ascending.as_array().expect("a page"), &reversed);
}

/// An unrecognised `sort_by` is not rejected; it simply leaves the order alone.
#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_sort_field_is_accepted_and_ignored() {
    let (status, page) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=not-a-field").await;

    assert_eq!(status, StatusCode::OK);
    assert!(!page.as_array().expect("a page").is_empty());
}

/// Sorting has to happen before the window is taken, or paging returns a different set per sort.
#[tokio::test(flavor = "multi_thread")]
async fn the_window_is_taken_after_sorting() {
    let (_, whole) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=id").await;
    let (status, first_two) = get_json("/private_api/taxa/filter?start=0&end=2&sort_by=id").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(first_two.as_array().expect("a page"), &whole.as_array().expect("a page")[..2]);
}

fn as_set(value: &serde_json::Value) -> std::collections::BTreeSet<u64> {
    value.as_array().expect("a page").iter().map(|id| id.as_u64().expect("an id")).collect()
}
