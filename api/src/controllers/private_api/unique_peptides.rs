//! # `private_api/taxa/unique_peptides`
//!
//! Reports peptides that are taxonomically specific to a given species- or strain-level taxon,
//! and — optionally — peptides that are specific to a broader parent clade containing that taxon.
//!
//! ## Overview
//!
//! Given a taxon ID at species or strain rank, the endpoint:
//!
//! 1. Fetches all UniProt protein sequences associated with `taxon_id` from the OpenSearch
//!    database.
//! 2. Performs an in-silico tryptic digest of each sequence using the provided (or default)
//!    cleavage regex, discarding fragments shorter than `min_length`.
//! 3. Looks up each distinct peptide in the global suffix-array index to find every protein in the
//!    entire UniProt database that contains it.
//! 4. Classifies each peptide into one of two categories:
//!    - **`unique_peptides`**: every protein that contains the peptide belongs to `taxon_id` or
//!      to a descendant taxon (e.g. a strain under a species). In other words, the LCA of all
//!      matching proteins is `taxon_id` or below, and no protein falls outside the subtree.
//!      These peptides are fully clade-specific and can be used as unambiguous markers.
//!    - **`unique_to_parent`** (only when `parent_taxon_id` is supplied): among the remaining
//!      non-unique peptides, those whose LCA of all matching taxa falls within the subtree of
//!      `parent_taxon_id`. Such peptides occur in more than one taxon, but all those taxa belong
//!      to the parent clade, so the peptide never occurs outside it.
//!
//! Peptides whose suffix-array search hit the match cutoff, or that returned no protein hits at
//! all, are excluded from both categories because their taxon membership cannot be reliably
//! determined.
//!
//! ## Intended use
//!
//! The `unique_to_parent` category is intended for web applications that construct a partial-
//! coverage peptide set for a parent taxon (e.g. a genus or family). Neither `unique_peptides` nor
//! `unique_to_parent` alone covers the full diversity of the parent clade; together they provide
//! every peptide that is confined to that clade and that originates from the requested taxon's
//! protein sequences.
//!
//! ## Request parameters
//!
//! | Parameter         | Type   | Required | Default        | Description |
//! |-------------------|--------|----------|----------------|-------------|
//! | `taxon_id`        | u32    | yes      | —              | NCBI taxon ID. Must be at species or strain rank. |
//! | `cleavage_regex`  | String | no       | `[KR](?!P)`    | Regex applied to protein sequences to determine cleavage sites (trypsin rule by default). Supports lookahead via `fancy_regex`. |
//! | `min_length`      | usize  | no       | `5`            | Minimum peptide length in amino acids. Shorter fragments are discarded before index lookup. |
//! | `parent_taxon_id` | u32    | no       | absent         | NCBI taxon ID of a clade that contains `taxon_id`. When provided, enables the `unique_to_parent` output. Must be a strict ancestor of `taxon_id` in the NCBI taxonomy lineage. |
//!
//! The endpoint accepts both GET (query string) and POST (JSON body).
//!
//! ## Validation
//!
//! - `taxon_id` must resolve to a taxon at species or strain rank in the taxon store; any other
//!   rank returns HTTP 400.
//! - When `parent_taxon_id` is supplied, it must appear in the lineage of `taxon_id` (i.e. it must
//!   be a direct or indirect ancestor). If it does not, the endpoint returns HTTP 400 with a
//!   descriptive message. This check is performed before the protein fetch to fail fast.
//!
//! ## Response fields
//!
//! | Field                          | Always present | Description |
//! |--------------------------------|----------------|-------------|
//! | `unique_peptides`              | yes            | Peptide sequences that occur exclusively in `taxon_id` across all of UniProt. |
//! | `total_peptides`               | yes            | Total number of distinct in-silico peptides generated from `taxon_id`'s proteins (before index filtering). |
//! | `total_unique_peptides`        | yes            | Length of `unique_peptides`. |
//! | `unique_to_parent`             | only with parent | Peptide sequences that are not fully unique to `taxon_id` but whose LCA (over all UniProt proteins containing the peptide) is equal to `parent_taxon_id` or is a descendant of it. |
//! | `total_unique_to_parent_peptides` | only with parent | Length of `unique_to_parent`. |
//!
//! `unique_peptides` and `unique_to_parent` are always disjoint: a peptide appears in at most one
//! of the two lists.
//!
//! ## LCA computation for `unique_to_parent`
//!
//! For each non-unique peptide (i.e. one whose proteins are not all from `taxon_id`), the LCA is
//! computed over the set of NCBI taxon IDs of all matching proteins via `calculate_lca`. Only valid
//! taxa are considered (`only_valid_taxa = true`). The LCA is then checked for membership in the
//! parent subtree using the same lineage-array approach as `TaxaFilter`: the LCA is considered
//! "within the parent subtree" when `lca_id == parent_taxon_id` or when `parent_taxon_id` appears
//! anywhere in the LCA's own lineage array (meaning the parent is an ancestor of the LCA, so the
//! LCA is a descendant of the parent).

use std::collections::HashMap;
use std::time::Instant;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    helpers::{
        digestion::{cleave_sequence, compile_cleavage_regex, default_cleavage_regex, default_min_length, validate_taxon_rank},
        lca_helper::calculate_lca,
        lineage_helper::{is_ancestor, LineageVersion},
    },
    AppState,
};
use database::get_protein_sequences_for_taxon;

#[derive(Deserialize)]
pub struct Parameters {
    /// NCBI taxon ID for which to compute unique peptides. Must be at species or strain rank.
    taxon_id: u32,
    /// Cleavage regex applied to protein sequences. Defaults to `[KR](?!P)` (trypsin).
    #[serde(default = "default_cleavage_regex")]
    cleavage_regex: String,
    /// Minimum peptide length in amino acids. Fragments shorter than this are discarded.
    #[serde(default = "default_min_length")]
    min_length: usize,
    /// Optional NCBI taxon ID of a parent clade. Must be a strict ancestor of `taxon_id`.
    /// When present, enables the `unique_to_parent` output: non-unique peptides whose LCA over
    /// all UniProt proteins falls within this clade are reported separately.
    #[serde(default)]
    parent_taxon_id: Option<u32>,
}

#[derive(Serialize)]
pub struct UniquePeptidesResult {
    /// Peptides whose every matching UniProt protein belongs to `taxon_id` or a descendant of it
    /// (e.g. a strain under a species). No protein containing the peptide falls outside the subtree.
    unique_peptides: Vec<String>,
    /// Total distinct in-silico peptides generated from `taxon_id`'s proteins.
    total_peptides: usize,
    /// Number of entries in `unique_peptides`.
    total_unique_peptides: usize,
    /// Non-unique peptides whose LCA of all matching UniProt proteins falls within the subtree of
    /// `parent_taxon_id`. Absent when `parent_taxon_id` was not supplied. Disjoint from
    /// `unique_peptides`.
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_to_parent: Option<Vec<String>>,
    /// Number of entries in `unique_to_parent`. Absent when `parent_taxon_id` was not supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    total_unique_to_parent_peptides: Option<usize>,
}

async fn handler(
    State(AppState { index, datastore, database, .. }): State<AppState>,
    Parameters { taxon_id, cleavage_regex, min_length, parent_taxon_id }: Parameters,
) -> Result<UniquePeptidesResult, ApiError> {
    let re = compile_cleavage_regex(&cleavage_regex)?;
    validate_taxon_rank(datastore.taxon_store(), taxon_id)?;

    // Validate that parent_taxon_id, when provided, is a strict ancestor of taxon_id.
    if let Some(parent) = parent_taxon_id {
        if !is_ancestor(parent, taxon_id, LineageVersion::V2, datastore.lineage_store()) {
            return Err(ApiError::InvalidParameterError(format!(
                "Parent taxon {} is not an ancestor of taxon {}",
                parent, taxon_id
            )));
        }
    }

    let t_start = Instant::now();

    let sequences = get_protein_sequences_for_taxon(database.get_conn(), taxon_id).await?;
    tracing::info!(taxon_id, proteins = sequences.len(), elapsed_ms = t_start.elapsed().as_millis(), "protein fetch complete");

    let t_digest = Instant::now();
    let mut peptides: Vec<String> = sequences.iter()
        .flat_map(|sequence| cleave_sequence(sequence, &re))
        .filter(|f| f.len() >= min_length)
        .collect();

    peptides.sort_unstable();
    peptides.dedup();

    let total_peptides = peptides.len();
    tracing::info!(taxon_id, peptides = total_peptides, elapsed_ms = t_digest.elapsed().as_millis(), "digestion and dedup complete");

    let t_index = Instant::now();
    let (peptides, results) = tokio::task::spawn_blocking(move || {
        let results = index.analyse(&peptides, false, false, Some(10_000));
        (peptides, results)
    }).await?;
    tracing::info!(taxon_id, elapsed_ms = t_index.elapsed().as_millis(), "suffix array search complete");

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let t_classify = Instant::now();
    let mut unique_peptides: Vec<String> = Vec::new();
    let mut unique_to_parent: Vec<String> = Vec::new();

    // Cache is_ancestor results keyed by protein taxon ID. The two fixed arguments (taxon_id and
    // parent) are the same for every call within a request, so we only need to compute once per
    // distinct protein taxon encountered in the result set.
    let mut descendant_of_taxon: HashMap<u32, bool> = HashMap::new();
    let mut in_parent_subtree: HashMap<u32, bool> = HashMap::new();

    for (peptide, result) in peptides.into_iter().zip(results) {
        // Skip peptides where the match set is unreliable: the cutoff was hit (too many proteins
        // matched, taxon membership cannot be determined) or no proteins were found at all.
        if result.cutoff_used || result.proteins.is_empty() {
            continue;
        }

        if result.proteins.iter().all(|p| {
            p.taxon == taxon_id || *descendant_of_taxon
                .entry(p.taxon)
                .or_insert_with(|| is_ancestor(taxon_id, p.taxon, LineageVersion::V2, lineage_store))
        }) {
            unique_peptides.push(peptide);
        } else if let Some(parent) = parent_taxon_id {
            // Compute the LCA of all taxa that contain this peptide. If the LCA is within the
            // parent subtree, the peptide never occurs outside the parent clade.
            let taxa: Vec<u32> = result.proteins.iter().map(|p| p.taxon).collect();
            let lca = calculate_lca(taxa, LineageVersion::V2, taxon_store, lineage_store, true);
            let lca_id = lca as u32;

            // The LCA is within the parent subtree when it equals the parent (the parent is
            // exactly the LCA) or when the parent is a strict ancestor of the LCA.
            // Note: a taxon's lineage array does not include itself, so the equality arm is needed.
            let lca_in_parent_subtree = lca_id == parent
                || *in_parent_subtree
                    .entry(lca_id)
                    .or_insert_with(|| is_ancestor(parent, lca_id, LineageVersion::V2, lineage_store));

            if lca_in_parent_subtree {
                unique_to_parent.push(peptide);
            }
        }
    }
    tracing::info!(taxon_id, elapsed_ms = t_classify.elapsed().as_millis(), "classification complete");
    tracing::info!(taxon_id, total_elapsed_ms = t_start.elapsed().as_millis(), "unique_peptides request complete");

    let total_unique_peptides = unique_peptides.len();

    let (unique_to_parent_field, total_unique_to_parent_field) = if parent_taxon_id.is_some() {
        let count = unique_to_parent.len();
        (Some(unique_to_parent), Some(count))
    } else {
        (None, None)
    };

    Ok(UniquePeptidesResult {
        unique_peptides,
        total_peptides,
        total_unique_peptides,
        unique_to_parent: unique_to_parent_field,
        total_unique_to_parent_peptides: total_unique_to_parent_field,
    })
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<UniquePeptidesResult>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
