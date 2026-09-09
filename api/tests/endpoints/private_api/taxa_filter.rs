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

/// `end` below `start` is a malformed request: the window has no size, so there is nothing to
/// return and no page to fall back on.
#[tokio::test(flavor = "multi_thread")]
async fn an_end_below_start_is_rejected() {
    let (dir, state) = offline_state();
    let request = Request::get("/private_api/taxa/filter?filter=&start=10&end=0").body(Body::empty()).unwrap();
    let (status, body) = request_raw(state, request).await;
    drop(dir);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("end"), "the message should name the parameter, got: {body}");
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

/// The filter still matches a rank as text, with a space and without regard to case.
///
/// Normalising the separator in `rank_to_idx` must not reach here: this endpoint matches a rank
/// name the way a user types it, and never asks for a lineage column.
#[tokio::test(flavor = "multi_thread")]
async fn a_rank_filter_still_matches_the_spaced_name_in_any_case() {
    let (_, spaced) = get_json("/private_api/taxa/count?filter=species%20group").await;
    let (_, shouted) = get_json("/private_api/taxa/count?filter=SPECIES%20GROUP").await;
    let (_, keyed) = get_json("/private_api/taxa/count?filter=species_group").await;

    assert_eq!(spaced["count"], 1);
    assert_eq!(shouted["count"], 1, "the filter ignores case");
    assert_eq!(keyed["count"], 0, "the filter matches the name, not the column key");
}

/// Many taxa carry the same rank, and the id settles every one of those ties, so the order is
/// total. Nothing else could settle them: the taxa are collected out of a `HashMap`, whose
/// iteration order differs from one process to the next.
///
/// A page is a window on this order. Two processes ordering the ties differently would hand a
/// client paging through the list one taxon twice and another not at all.
#[tokio::test(flavor = "multi_thread")]
async fn taxa_of_one_rank_are_ordered_by_id() {
    let (status, page) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=rank").await;
    assert_eq!(status, StatusCode::OK);

    let ids: Vec<u64> = page.as_array().expect("a page").iter().map(|id| id.as_u64().expect("an id")).collect();

    let (_, ranked) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=id").await;
    assert_eq!(ids.len(), ranked.as_array().expect("a page").len(), "the same taxa, differently arranged");

    // Ids ascend within each run of one rank, which is what a tiebreak on the id means from here.
    let mut ties = 0;
    for pair in ids.windows(2) {
        let (_, first) = get_json(&format!("/private_api/taxa?taxids[]={}", pair[0])).await;
        let (_, second) = get_json(&format!("/private_api/taxa?taxids[]={}", pair[1])).await;

        if first[0]["rank"] == second[0]["rank"] {
            ties += 1;
            assert!(pair[0] < pair[1], "{} and {} share a rank but are not in id order", pair[0], pair[1]);
        }
    }

    assert!(ties > 0, "the corpus should hold at least two taxa of one rank, or this asserts nothing");
}

/// Descending reverses the whole order, the tiebreak included, so the two directions are exact
/// mirrors even where the sort field repeats.
#[tokio::test(flavor = "multi_thread")]
async fn sorting_by_rank_reverses_exactly() {
    let (_, ascending) = get_json("/private_api/taxa/filter?start=0&end=100&sort_by=rank").await;
    let (status, descending) =
        get_json("/private_api/taxa/filter?start=0&end=100&sort_by=rank&sort_descending=true").await;

    assert_eq!(status, StatusCode::OK);

    let mut reversed = descending.as_array().expect("a page").clone();
    reversed.reverse();
    assert_eq!(ascending.as_array().expect("a page"), &reversed);
}

/// Every window is the slice of the whole listing that sits at the same offsets, on each sort field
/// and in both directions.
///
/// A page is taken by partitioning around its bounds rather than by ordering the whole table, so
/// what is asserted is that the cheaper route answers exactly what the ordering would have.
#[tokio::test(flavor = "multi_thread")]
async fn every_window_matches_the_whole_listing() {
    // `rank` is the field many taxa share, so it is where a partition could pick differently from a
    // sort; `id` covers the branch that needs no tiebreak.
    for field in ["id", "rank"] {
        for descending in ["false", "true"] {
            let sorted = format!("sort_by={field}&sort_descending={descending}");
            let (status, whole) = get_json(&format!("/private_api/taxa/filter?start=0&end=1000&{sorted}")).await;

            assert_eq!(status, StatusCode::OK, "{sorted}");
            let whole = whole.as_array().expect("a page");
            assert!(whole.len() > 3, "{sorted}: too few taxa to page through");

            for start in 0..whole.len() {
                for size in [1usize, 3] {
                    let end = start + size;
                    let (_, window) =
                        get_json(&format!("/private_api/taxa/filter?start={start}&end={end}&{sorted}")).await;

                    let expected: Vec<_> = whole.iter().skip(start).take(size).cloned().collect();
                    assert_eq!(
                        window.as_array().expect("a page"),
                        &expected,
                        "{sorted}: the window [{start}, {end}) differs from the same slice of the listing"
                    );
                }
            }
        }
    }
}

/// Pages walked end to end rebuild the whole listing, so every taxon on one page sorts below every
/// taxon on the next.
///
/// A page is cut by partitioning around its bounds rather than by ordering the whole table. What
/// makes the two agree is that the comparison is a total order: no two taxa compare equal, so "the
/// taxa at ranks [start, end)" names one set of taxa, whichever way the partition reached it.
///
/// The listing compared against is a genuine sort, not another partition — a window wider than the
/// table skips both partition steps.
#[tokio::test(flavor = "multi_thread")]
async fn pages_walked_end_to_end_rebuild_the_listing() {
    for field in ["id", "rank", "name"] {
        for descending in ["false", "true"] {
            let sorted = format!("sort_by={field}&sort_descending={descending}");
            let (_, whole) = get_json(&format!("/private_api/taxa/filter?start=0&end=1000&{sorted}")).await;
            let whole = whole.as_array().expect("a page").clone();

            let mut walked = Vec::new();
            let mut start = 0;
            while start < whole.len() {
                let (_, page) =
                    get_json(&format!("/private_api/taxa/filter?start={start}&end={}&{sorted}", start + 3)).await;
                walked.extend(page.as_array().expect("a page").clone());
                start += 3;
            }

            assert_eq!(walked, whole, "{sorted}: pages walked end to end differ from the whole listing");
        }
    }
}
