//! `/private_api/proteomes/count` and `/proteomes/filter` — counting and paging the proteomes.
//!
//! The two share a filter and have to agree about what it selects, which is the property worth
//! testing rather than either number alone.

use axum::{
    body::Body,
    http::{Request, StatusCode}
};

use crate::common::{get_json, offline_state, request_raw};

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_filter_counts_every_proteome() {
    let (status, body) = get_json("/private_api/proteomes/count").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], 4, "the corpus declares four reference proteomes");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_filter_selects_fewer_than_everything() {
    let (_, all) = get_json("/private_api/proteomes/count").await;
    let (status, filtered) = get_json("/private_api/proteomes/count?filter=UP000000001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(filtered["count"], 1);
    assert!(filtered["count"].as_u64() < all["count"].as_u64());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_filter_matching_nothing_counts_nothing() {
    let (status, body) = get_json("/private_api/proteomes/count?filter=no-such-proteome").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_returns_at_most_what_it_asked_for() {
    let (status, page) = get_json("/private_api/proteomes/filter?start=0&end=2").await;

    assert_eq!(status, StatusCode::OK);
    assert!(page.as_array().expect("a page").len() <= 2);
}

/// The count and the listing have to agree, since a client pages through one using the other.
#[tokio::test(flavor = "multi_thread")]
async fn the_count_matches_what_paging_yields() {
    let (_, counted) = get_json("/private_api/proteomes/count").await;
    let (status, listed) = get_json("/private_api/proteomes/filter?start=0&end=100").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().map(Vec::len), counted["count"].as_u64().map(|n| n as usize));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_past_the_end_is_empty_rather_than_an_error() {
    let (status, page) = get_json("/private_api/proteomes/filter?start=500&end=600").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(page.as_array().map(Vec::len), Some(0));
}

/// `start` and `end` are required here, unlike `filter`.
/// The rejection body is plain text rather than JSON, so this reads it raw.
#[tokio::test(flavor = "multi_thread")]
async fn the_window_bounds_are_required() {
    let (dir, state) = offline_state();
    let request = Request::get("/private_api/proteomes/filter").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, "invalid query string");
}

/// `end` below `start` is a malformed request: the window has no size, so there is nothing to
/// return and no page to fall back on.
#[tokio::test(flavor = "multi_thread")]
async fn an_end_below_start_is_rejected() {
    let (dir, state) = offline_state();
    let request = Request::get("/private_api/proteomes/filter?filter=&start=10&end=0").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("end"), "the message should name the parameter, got: {body}");
}

// ── sorting ─────────────────────────────────────────────────────────────────────────────────────
//
// The comment on `sort_by` says "id", "name" or "rank", but the handler matches on `taxon_name` and
// `protein_count`. Both spellings are exercised below, because the difference between what the
// parameter documents and what it does is not visible from the outside otherwise.

#[tokio::test(flavor = "multi_thread")]
async fn sorting_by_id_orders_and_reverses() {
    let (status, ascending) = get_json("/private_api/proteomes/filter?start=0&end=100&sort_by=id").await;
    assert_eq!(status, StatusCode::OK);

    let ids: Vec<&str> = ascending.as_array().expect("a page").iter().map(|i| i.as_str().unwrap()).collect();
    assert!(ids.windows(2).all(|pair| pair[0] <= pair[1]), "not ascending: {ids:?}");

    let (_, descending) =
        get_json("/private_api/proteomes/filter?start=0&end=100&sort_by=id&sort_descending=true").await;
    let got: Vec<&str> = descending.as_array().expect("a page").iter().map(|i| i.as_str().unwrap()).collect();
    let mut reversed = ids.clone();
    reversed.reverse();
    assert_eq!(got, reversed);
}

/// The fields the handler actually matches on, which are not the ones its comment names.
#[tokio::test(flavor = "multi_thread")]
async fn sorting_by_taxon_name_and_protein_count_reorders_without_losing_anything() {
    let (_, baseline) = get_json("/private_api/proteomes/filter?start=0&end=100").await;
    let expected = as_set(&baseline);

    for field in ["taxon_name", "protein_count"] {
        for descending in ["false", "true"] {
            let path =
                format!("/private_api/proteomes/filter?start=0&end=100&sort_by={field}&sort_descending={descending}");
            let (status, page) = get_json(&path).await;

            assert_eq!(status, StatusCode::OK, "{field}/{descending}");
            assert_eq!(as_set(&page), expected, "sorting by {field} must not change which proteomes come back");
        }
    }
}

/// `name` and `rank` are what the parameter's comment advertises, and neither is a case the handler
/// matches — so they sort nothing. Recorded rather than corrected: making them work is a change to
/// what the endpoint accepts.
#[tokio::test(flavor = "multi_thread")]
async fn the_documented_sort_fields_are_not_the_implemented_ones() {
    let (_, unsorted) = get_json("/private_api/proteomes/filter?start=0&end=100&sort_by=not-a-field").await;
    let (status, by_name) = get_json("/private_api/proteomes/filter?start=0&end=100&sort_by=name").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(by_name, unsorted, "`name` is accepted and does nothing");
}

fn as_set(value: &serde_json::Value) -> std::collections::BTreeSet<String> {
    value.as_array().expect("a page").iter().map(|id| id.as_str().expect("an id").to_string()).collect()
}

/// `UP000000002` and `UP000000004` share a taxon and a protein count, so on both sort fields the
/// proteome id is what separates them, and the order is total.
///
/// Nothing else could separate them: the proteomes are collected out of a `HashMap`, whose
/// iteration order differs from one process to the next. A page is a window on this order, so two
/// processes ordering the pair differently would hand a client paging through the list one proteome
/// twice and another not at all.
#[tokio::test(flavor = "multi_thread")]
async fn proteomes_alike_on_the_sort_field_are_ordered_by_id() {
    for field in ["taxon_name", "protein_count"] {
        let path = format!("/private_api/proteomes/filter?start=0&end=100&sort_by={field}");
        let (status, page) = get_json(&path).await;

        assert_eq!(status, StatusCode::OK, "{field}");

        let ids: Vec<&str> =
            page.as_array().expect("a page").iter().map(|id| id.as_str().expect("an accession")).collect();
        let second = ids.iter().position(|id| *id == "UP000000002").expect("UP000000002 is in the page");
        let fourth = ids.iter().position(|id| *id == "UP000000004").expect("UP000000004 is in the page");

        assert_eq!(fourth, second + 1, "sorted by {field} the tied pair is adjacent, in id order: {ids:?}");
    }
}

/// Descending reverses the whole order, the tiebreak included, so the two directions are exact
/// mirrors even on a field two proteomes share.
#[tokio::test(flavor = "multi_thread")]
async fn sorting_by_protein_count_reverses_exactly() {
    let (ascending_status, ascending) =
        get_json("/private_api/proteomes/filter?start=0&end=100&sort_by=protein_count").await;
    let (descending_status, descending) =
        get_json("/private_api/proteomes/filter?start=0&end=100&sort_by=protein_count&sort_descending=true").await;

    assert_eq!(ascending_status, StatusCode::OK, "ascending");
    assert_eq!(descending_status, StatusCode::OK, "descending");

    let mut reversed = descending.as_array().expect("a page").clone();
    reversed.reverse();
    assert_eq!(ascending.as_array().expect("a page"), &reversed);
}
