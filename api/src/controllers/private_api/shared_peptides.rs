use std::collections::HashSet;
use std::sync::Arc;

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
    #[serde(default)]
    taxon_ids: Vec<u32>,
    #[serde(default = "default_cleavage_regex")]
    cleavage_regex: String,
    #[serde(default = "default_min_length")]
    min_length: usize,
}

#[derive(Serialize)]
pub struct SharedPeptidesResult {
    shared_peptides: Vec<String>,
}

async fn handler(
    State(AppState { datastore, database, .. }): State<AppState>,
    Parameters { taxon_ids, cleavage_regex, min_length }: Parameters,
) -> Result<SharedPeptidesResult, ApiError> {
    if taxon_ids.is_empty() {
        return Ok(SharedPeptidesResult { shared_peptides: vec![] });
    }

    let re = Arc::new(compile_cleavage_regex(&cleavage_regex)?);

    for &taxon_id in &taxon_ids {
        validate_taxon_rank(datastore.taxon_store(), taxon_id)?;
    }

    let n = taxon_ids.len();
    let mut join_set = tokio::task::JoinSet::<Result<HashSet<String>, ApiError>>::new();

    for taxon_id in taxon_ids {
        let database = database.clone();
        let re = Arc::clone(&re);
        join_set.spawn(async move {
            let proteins = get_proteins_for_taxon(database.get_conn(), taxon_id).await?;
            let peptides: HashSet<String> = proteins.iter()
                .flat_map(|p| cleave_sequence(&p.protein, &re))
                .filter(|f| f.len() >= min_length)
                .collect();
            Ok(peptides)
        });
    }

    let mut sets: Vec<HashSet<String>> = Vec::with_capacity(n);
    while let Some(result) = join_set.join_next().await {
        sets.push(result.map_err(ApiError::JoinError)??);
    }

    let mut shared_peptides: Vec<String> = sets
        .into_iter()
        .reduce(|acc, set| acc.into_iter().filter(|p| set.contains(p)).collect())
        .unwrap_or_default()
        .into_iter()
        .collect();
    shared_peptides.sort_unstable();

    Ok(SharedPeptidesResult { shared_peptides })
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<SharedPeptidesResult>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
