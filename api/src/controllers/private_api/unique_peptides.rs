use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    helpers::digestion::{cleave_sequence, compile_cleavage_regex, default_cleavage_regex, default_min_length, validate_taxon_rank},
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
}

#[derive(Serialize)]
pub struct UniquePeptidesResult {
    unique_peptides: Vec<String>,
    total_peptides: usize,
    total_unique_peptides: usize,
}

async fn handler(
    State(AppState { index, datastore, database, .. }): State<AppState>,
    Parameters { taxon_id, cleavage_regex, min_length }: Parameters,
) -> Result<UniquePeptidesResult, ApiError> {
    let re = compile_cleavage_regex(&cleavage_regex)?;
    validate_taxon_rank(datastore.taxon_store(), taxon_id)?;

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

    let unique_peptides: Vec<String> = peptides.into_iter()
        .zip(results)
        .filter_map(|(peptide, result)| {
            (!result.cutoff_used
                && !result.proteins.is_empty()
                && result.proteins.iter().all(|p| p.taxon == taxon_id))
            .then_some(peptide)
        })
        .collect();

    let total_unique_peptides = unique_peptides.len();

    Ok(UniquePeptidesResult {
        unique_peptides,
        total_peptides,
        total_unique_peptides,
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
