//! # `private_api/taxa/unique_peptides`
//!
//! Reports peptides that are taxonomically specific to a given species- or strain-level taxon,
//! and — optionally — peptides that are specific to a broader parent clade containing that taxon.
//!
//! ## Overview
//!
//! Given a taxon ID at species or strain rank and a half-open protein range `[start, end)`, the
//! endpoint:
//!
//! 1. Fetches the UniProt protein sequences at positions `[start, end)` in the
//!    `uniprot_accession_number`-sorted list for `taxon_id` from the OpenSearch database.
//! 2. Performs an in-silico tryptic digest of each sequence using the provided (or default)
//!    cleavage regex, discarding fragments shorter than `min_length`.
//! 3. Looks up each distinct peptide in the global suffix-array index to find every protein in the
//!    entire UniProt database that contains it.
//! 4. Classifies each peptide into one of two categories:
//!    - **`unique_peptides`**: every protein that contains the peptide belongs to `taxon_id` or
//!      to a descendant taxon (e.g. a strain under a species). In other words, the LCA of all
//!      matching proteins is `taxon_id` or below, and no protein falls outside the subtree.
//!      These peptides are fully clade-specific and can be used as unambiguous markers.
//!    - **`unique_to_parent`** (only when `parent_id` is supplied): among the remaining
//!      non-unique peptides, those whose LCA of all matching taxa falls within the subtree of
//!      `parent_id`. Such peptides occur in more than one taxon, but all those taxa belong
//!      to the parent clade, so the peptide never occurs outside it.
//!
//! Peptides whose suffix-array search hit the match cutoff, or that returned no protein hits at
//! all, are excluded from both categories because their taxon membership cannot be reliably
//! determined.
//!
//! ## Batched consumption
//!
//! Use the companion count endpoint (`/taxa/unique_peptides/count?taxon_id=X`) to retrieve the
//! total number of proteins for a taxon, then fan out parallel requests over disjoint ranges
//! (e.g. `[0, 1000)`, `[1000, 2000)`, …). Because peptide classification depends only on the
//! suffix-array index — not on the protein slice — the same peptide receives the same
//! classification in every batch that produces it. Clients therefore take the **union** of
//! `unique_peptides` (and `unique_to_parent`) across all batches and deduplicate. The union is
//! correct. `total_unique_peptides` / `total_unique_to_parent_peptides` in each response are
//! the lengths of that batch's result lists only; to get a global count, dedup the unioned sets.
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
//! | `start`           | usize  | yes      | —              | Start index (inclusive) of the protein range, in `uniprot_accession_number` order. |
//! | `end`             | usize  | yes      | —              | End index (exclusive) of the protein range. |
//! | `cleavage_regex`  | String | no       | `[KR](?!P)`    | Regex applied to protein sequences to determine cleavage sites (trypsin rule by default). Supports lookahead via `fancy_regex`. |
//! | `min_length`      | usize  | no       | `5`            | Minimum peptide length in amino acids. Shorter fragments are discarded before index lookup. |
//! | `parent_id`       | u32    | no       | absent         | NCBI taxon ID of a clade that contains `taxon_id`. When provided, enables the `unique_to_parent` output. Must be a strict ancestor of `taxon_id` in the NCBI taxonomy lineage. |
//!
//! The endpoint accepts both GET (query string) and POST (JSON body).
//!
//! ## Validation
//!
//! - `taxon_id` must resolve to a taxon at species or strain rank in the taxon store; any other
//!   rank returns HTTP 400.
//! - `start` and `end` are required; omitting either returns HTTP 400 (deserialization error).
//! - When `parent_id` is supplied, it must appear in the lineage of `taxon_id` (i.e. it must
//!   be a direct or indirect ancestor). If it does not, the endpoint returns HTTP 400 with a
//!   descriptive message. This check is performed before the protein fetch to fail fast.
//!
//! ## Response fields
//!
//! | Field                             | Always present   | Description |
//! |-----------------------------------|------------------|-------------|
//! | `unique_peptides`                 | yes              | Peptide sequences that occur exclusively in `taxon_id` across all of UniProt. |
//! | `total_unique_peptides`           | yes              | Length of `unique_peptides` for this batch. |
//! | `unique_to_parent`                | only with parent | Peptide sequences that are not fully unique to `taxon_id` but whose LCA (over all UniProt proteins containing the peptide) is equal to `parent_id` or is a descendant of it. |
//! | `total_unique_to_parent_peptides` | only with parent | Length of `unique_to_parent` for this batch. |
//!
//! `unique_peptides` and `unique_to_parent` are always disjoint: a peptide appears in at most one
//! of the two lists.
//!
//! ## Classification for `unique_to_parent`
//!
//! For each non-unique peptide, the endpoint checks whether every valid-taxon protein hit falls
//! within the subtree of `parent_id`. The subtree is precomputed as a `HashSet<u32>` by
//! scanning the lineage store once: any taxon whose lineage contains `parent_id` (via
//! `Lineage::contains_ancestor`) is a member. Membership checks during classification are then O(1)
//! per taxon, with no LCA computation needed.

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
use database::{get_protein_counts_by_taxon_ids, get_protein_sequences_for_taxon_range};

/// Parameters for the count endpoint (`/taxa/unique_peptides/count`).
#[derive(Deserialize)]
pub struct CountParameters {
    /// NCBI taxon ID. Must be at species or strain rank.
    taxon_id: u32,
}

/// Response for the count endpoint.
#[derive(Serialize)]
pub struct ProteinCountResult {
    /// Total number of UniProt proteins whose `taxon_id` field exactly matches the requested
    /// taxon. This is the value to use when computing `[start, end)` batch ranges for the
    /// `/taxa/unique_peptides` endpoint.
    protein_count: u32,
}

#[derive(Deserialize)]
pub struct Parameters {
    /// NCBI taxon ID for which to compute unique peptides. Must be at species or strain rank.
    taxon_id: u32,
    /// Start index (inclusive) of the protein range, in `uniprot_accession_number` order.
    start: usize,
    /// End index (exclusive) of the protein range.
    end: usize,
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
    parent_id: Option<u32>,
}

#[derive(Serialize)]
pub struct UniquePeptidesResult {
    /// Peptides whose every matching UniProt protein belongs to `taxon_id` or a descendant of it
    /// (e.g. a strain under a species). No protein containing the peptide falls outside the subtree.
    unique_peptides: Vec<String>,
    /// Number of entries in `unique_peptides` for this batch.
    total_unique_peptides: usize,
    /// Non-unique peptides whose LCA of all matching UniProt proteins falls within the subtree of
    /// `parent_id`. Absent when `parent_id` was not supplied. Disjoint from `unique_peptides`.
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_to_parent: Option<Vec<String>>,
    /// Number of entries in `unique_to_parent` for this batch. Absent when `parent_id` was not supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    total_unique_to_parent_peptides: Option<usize>,
}

async fn count_handler(
    State(AppState { datastore, database, .. }): State<AppState>,
    CountParameters { taxon_id }: CountParameters,
) -> Result<ProteinCountResult, ApiError> {
    validate_taxon_rank(datastore.taxon_store(), taxon_id)?;

    let counts = get_protein_counts_by_taxon_ids(database.get_conn(), &[taxon_id as i32]).await?;
    let protein_count = *counts.get(&taxon_id).unwrap_or(&0);

    Ok(ProteinCountResult { protein_count })
}

async fn handler(
    State(AppState { index, datastore, database, .. }): State<AppState>,
    Parameters { taxon_id, start, end, cleavage_regex, min_length, parent_id }: Parameters,
) -> Result<UniquePeptidesResult, ApiError> {
    let re = compile_cleavage_regex(&cleavage_regex)?;
    validate_taxon_rank(datastore.taxon_store(), taxon_id)?;

    // Validate that parent_id, when provided, is a strict ancestor of taxon_id.
    if let Some(parent) = parent_id {
        if !is_ancestor(parent, taxon_id, LineageVersion::V2, datastore.lineage_store()) {
            return Err(ApiError::InvalidParameterError(format!(
                "Parent taxon {} is not an ancestor of taxon {}",
                parent, taxon_id
            )));
        }
    }

    let t_start = Instant::now();

    let sequences = get_protein_sequences_for_taxon_range(database.get_conn(), taxon_id, start, end).await?;
    tracing::info!(taxon_id, start, end, proteins = sequences.len(), elapsed_ms = t_start.elapsed().as_millis(), "protein fetch complete");

    let t_digest = Instant::now();
    let mut peptides: Vec<String> = sequences.iter()
        .flat_map(|sequence| cleave_sequence(sequence, &re))
        .filter(|f| f.len() >= min_length)
        .collect();

    peptides.sort_unstable();
    peptides.dedup();

    tracing::info!(taxon_id, peptides = peptides.len(), elapsed_ms = t_digest.elapsed().as_millis(), "digestion and dedup complete");

    let t_index = Instant::now();
    let (peptides, results) = tokio::task::spawn_blocking(move || {
        let results = index.analyse_taxa(&peptides, false, false, Some(10_000));
        (peptides, results)
    }).await?;
    tracing::info!(taxon_id, elapsed_ms = t_index.elapsed().as_millis(), "suffix array search complete");

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let t_classify = Instant::now();

    // Precompute descendant sets by scanning the lineage store once per set. Each set includes the
    // root taxon itself plus every taxon whose lineage contains it as an ancestor.
    let descendant_set: HashSet<u32> = std::iter::once(taxon_id)
        .chain(lineage_store.mapper.iter().filter_map(|(tid, lin)| {
            if lin.contains_ancestor(taxon_id) { Some(*tid) } else { None }
        }))
        .collect();

    let parent_descendant_set: Option<HashSet<u32>> = parent_id.map(|parent| {
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

    let (unique_to_parent_field, total_unique_to_parent_field) = if parent_id.is_some() {
        let count = unique_to_parent.len();
        (Some(unique_to_parent), Some(count))
    } else {
        (None, None)
    };

    Ok(UniquePeptidesResult {
        unique_peptides,
        total_unique_peptides,
        unique_to_parent: unique_to_parent_field,
        total_unique_to_parent_peptides: total_unique_to_parent_field,
    })
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => CountParameters
    ) -> Result<Json<ProteinCountResult>, ApiError> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<UniquePeptidesResult>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
