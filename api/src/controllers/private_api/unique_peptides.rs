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

use std::collections::HashSet;
use std::time::Instant;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    helpers::{
        digestion::{cleave_sequence, compile_cleavage_regex, default_cleavage_regex, default_min_length, validate_taxon_rank},
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
        let results = index.analyse_taxa(&peptides, false, false, Some(10_000));
        (peptides, results)
    }).await?;
    tracing::info!(taxon_id, elapsed_ms = t_index.elapsed().as_millis(), "suffix array search complete");

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let t_classify = Instant::now();

    // Build the subtree set for taxon_id once. Since taxon_id is validated to be at species or
    // strain rank, every descendant will have taxon_id in exactly the species or strain field of
    // its lineage. Scanning the entire lineage store once is far cheaper than doing per-protein
    // is_ancestor calls (with or without memoisation) across potentially millions of protein hits.
    let descendant_set: HashSet<u32> = std::iter::once(taxon_id)
        .chain(lineage_store.mapper.iter().filter_map(|(tid, lin)| {
            let in_subtree = lin.species.map_or(false, |v| v.unsigned_abs() == taxon_id)
                || lin.strain.map_or(false, |v| v.unsigned_abs() == taxon_id);
            if in_subtree { Some(*tid) } else { None }
        }))
        .collect();

    // If parent_taxon_id is set, build its subtree set the same way. The parent may be at any
    // rank, so we check all lineage fields via contains_ancestor.
    let parent_descendant_set: Option<HashSet<u32>> = parent_taxon_id.map(|parent| {
        std::iter::once(parent)
            .chain(lineage_store.mapper.iter().filter_map(|(tid, lin)| {
                if lin.contains_ancestor(parent) { Some(*tid) } else { None }
            }))
            .collect()
    });

    tracing::info!(
        taxon_id,
        subtree_taxa = descendant_set.len(),
        elapsed_ms = t_classify.elapsed().as_millis(),
        "subtree sets built"
    );

    let mut unique_peptides: Vec<String> = Vec::new();
    let mut unique_to_parent: Vec<String> = Vec::new();

    let mut total_proteins_iterated: u64 = 0;

    for (peptide, result) in peptides.into_iter().zip(results) {
        if result.cutoff_used || result.taxa.is_empty() {
            continue;
        }

        // result.taxa is already sorted and deduplicated by search_all_taxa, so each taxon
        // appears at most once — no per-peptide cache is needed.
        total_proteins_iterated += result.taxa.len() as u64;

        if result.taxa.iter().all(|&t| descendant_set.contains(&t)) {
            unique_peptides.push(peptide);
        } else if let Some(ref parent_set) = parent_descendant_set {
            let mut has_valid = false;
            let all_in_parent = result.taxa.iter()
                .filter(|&&t| taxon_store.is_valid(t))
                .all(|&t| {
                    has_valid = true;
                    parent_set.contains(&t)
                });
            if has_valid && all_in_parent {
                unique_to_parent.push(peptide);
            }
        }
    }
    tracing::info!(
        taxon_id,
        total_proteins_iterated,
        elapsed_ms = t_classify.elapsed().as_millis(),
        "classification complete"
    );
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
