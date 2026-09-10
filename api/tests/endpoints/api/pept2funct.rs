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

/// `total_protein_count` means the same thing on all five endpoints that carry it.
///
/// The four functional endpoints used to report `fa.counts["all"]` — the number of distinct
/// proteins carrying at least one annotation. That undercounts whenever a matched protein has no
/// EC, GO or InterPro term, and drops to 0 when none of them has any. `peptinfo` was corrected to
/// `proteins.len()` on its own and the other four were left behind, so one field of the response
/// contract meant two different things.
///
/// `VALIDATION_SHARED` is the peptide that separates them: it matches two proteins, only one of
/// which the fixtures annotate, so the old expression answered 1 where `peptinfo` answered 2.
/// `UNIQUE` does not separate them — its one protein is annotated — which is why both are here.
///
/// The non-zero assertion keeps this honest: `fa.counts["all"]` reaches 0 when no matched protein
/// is annotated at all, and without it two endpoints both answering 0 would agree.
#[tokio::test(flavor = "multi_thread")]
async fn total_protein_count_means_matched_proteins_on_every_endpoint() {
    for peptide in [UNIQUE, VALIDATION_SHARED] {
        let (status, info) = get_json(&format!("/api/v2/peptinfo?input[]={peptide}")).await;
        assert_eq!(status, StatusCode::OK);

        let matched = info[0]["total_protein_count"].as_u64().expect("peptinfo reports a count");
        assert!(matched > 0, "`{peptide}` must match a protein for this to check anything");

        for endpoint in ["pept2ec", "pept2go", "pept2interpro", "pept2funct"] {
            let (status, body) = get_json(&format!("/api/v2/{endpoint}?input[]={peptide}")).await;

            assert_eq!(status, StatusCode::OK, "{endpoint}");
            assert_eq!(
                body[0]["total_protein_count"].as_u64(),
                Some(matched),
                "{endpoint} disagrees with peptinfo on `{peptide}`"
            );
        }
    }
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

#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_peptide_answers_at_each_position() {
    super::a_repeat_answers_like_a_single("pept2funct", "extra=true&domains=true").await;
}
