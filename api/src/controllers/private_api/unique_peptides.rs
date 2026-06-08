use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    helpers::{
        digestion::{cleave_sequence, compile_cleavage_regex, default_cleavage_regex, default_min_length, validate_taxon_rank},
        lca_helper::calculate_lca,
        lineage_helper::{get_lineage_array, LineageVersion},
    },
    AppState,
};
use database::get_proteins_for_taxon;

#[derive(Deserialize)]
pub struct Parameters {
    taxon_id: u32,
    #[serde(default = "default_cleavage_regex")]
    cleavage_regex: String,
    #[serde(default = "default_min_length")]
    min_length: usize,
    /// Optional parent taxon. When supplied, peptides that are not unique to `taxon_id` but whose
    /// LCA falls within the subtree of `parent_taxon_id` are reported as `unique_to_parent`.
    #[serde(default)]
    parent_taxon_id: Option<u32>,
}

#[derive(Serialize)]
pub struct UniquePeptidesResult {
    unique_peptides: Vec<String>,
    total_peptides: usize,
    total_unique_peptides: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_to_parent: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_unique_to_parent_peptides: Option<usize>,
}

async fn handler(
    State(AppState { index, datastore, database, .. }): State<AppState>,
    Parameters { taxon_id, cleavage_regex, min_length, parent_taxon_id }: Parameters,
) -> Result<UniquePeptidesResult, ApiError> {
    let re = compile_cleavage_regex(&cleavage_regex)?;
    validate_taxon_rank(datastore.taxon_store(), taxon_id)?;

    // Validate that parent_taxon_id, when provided, is an ancestor of taxon_id (or equal to it).
    // The lineage array of taxon_id contains every ancestor at every canonical rank; negative
    // values are no-rank placeholders and are compared via unsigned_abs(), matching TaxaFilter.
    if let Some(parent) = parent_taxon_id {
        let lineage_store = datastore.lineage_store();
        let is_ancestor = get_lineage_array(taxon_id, LineageVersion::V2, lineage_store)
            .iter()
            .flatten()
            .any(|a| a.unsigned_abs() == parent);
        if !is_ancestor {
            return Err(ApiError::InvalidParameterError(format!(
                "Parent taxon {} is not an ancestor of taxon {}",
                parent, taxon_id
            )));
        }
    }

    let proteins = get_proteins_for_taxon(database.get_conn(), taxon_id).await?;

    let mut peptides: Vec<String> = proteins.iter()
        .flat_map(|protein| cleave_sequence(&protein.protein, &re))
        .filter(|f| f.len() >= min_length)
        .collect();

    peptides.sort_unstable();
    peptides.dedup();

    let total_peptides = peptides.len();

    let (peptides, results) = tokio::task::spawn_blocking(move || {
        let results = index.analyse(&peptides, false, false, Some(10_000));
        (peptides, results)
    }).await?;

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let mut unique_peptides: Vec<String> = Vec::new();
    let mut unique_to_parent: Vec<String> = Vec::new();

    for (peptide, result) in peptides.into_iter().zip(results) {
        if result.cutoff_used || result.proteins.is_empty() {
            continue;
        }

        if result.proteins.iter().all(|p| p.taxon == taxon_id) {
            unique_peptides.push(peptide);
        } else if let Some(parent) = parent_taxon_id {
            let taxa: Vec<u32> = result.proteins.iter().map(|p| p.taxon).collect();
            let lca = calculate_lca(taxa, LineageVersion::V2, taxon_store, lineage_store, true);
            let lca_id = lca as u32;

            // The LCA is within the parent subtree when it equals the parent or when the parent
            // appears in the LCA's own lineage (i.e. the LCA descends from the parent).
            let lca_in_parent_subtree = lca_id == parent
                || get_lineage_array(lca_id, LineageVersion::V2, lineage_store)
                    .iter()
                    .flatten()
                    .any(|a| a.unsigned_abs() == parent);

            if lca_in_parent_subtree {
                unique_to_parent.push(peptide);
            }
        }
    }

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
