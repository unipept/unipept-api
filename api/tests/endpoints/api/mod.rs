//! `/api/v2/…` — the public endpoints.

use axum::http::StatusCode;
use fixtures::peptides::*;

use crate::common::get_json;

mod pept2ec;
mod pept2funct;
mod pept2go;
mod pept2interpro;
mod pept2lca;
mod pept2prot;
mod pept2taxa;
mod peptinfo;
mod protinfo;
mod taxa2lca;
mod taxa2tree;
mod taxonomy;

/// Asserts that a peptide named twice is answered at both positions, exactly as it is answered when
/// named alone.
///
/// Every peptide endpoint searches each distinct peptide once, so each of them needs this; the
/// query differs only in the route and its parameters.
pub async fn a_repeat_answers_like_a_single(route: &str, parameters: &str) {
    let single = format!("/api/v2/{route}?input[]={UNIQUE}&{parameters}");
    let repeated = format!("/api/v2/{route}?input[]={UNIQUE}&input[]={GENUS_SHARED}&input[]={UNIQUE}&{parameters}");

    let (status, once) = get_json(&single).await;
    assert_eq!(status, StatusCode::OK, "{single}");

    let (status, body) = get_json(&repeated).await;
    assert_eq!(status, StatusCode::OK, "{repeated}");

    let rows = body.as_array().expect("a list of results");
    assert_eq!(rows.len(), 3, "one row per position: {body}");
    assert_eq!(rows[0], once[0], "{route}: the first occurrence answers as it does alone");
    assert_eq!(rows[2], once[0], "{route}: and so does the second, to the byte");
    assert_eq!(rows[1]["peptide"], GENUS_SHARED, "{route}: the peptide between them keeps its place");
}
