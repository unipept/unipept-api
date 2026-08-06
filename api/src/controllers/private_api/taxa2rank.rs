use std::collections::HashMap;

use axum::{extract::State, Json};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use datastore::LineageStore;
use crate::{
    controllers::generate_handlers,
    helpers::lineage_helper::{get_lineage_array, LineageVersion},
    AppState
};
use crate::errors::ApiError;

#[derive(Deserialize)]
pub struct Parameters {
    /// Vector of taxa vectors, one per peptide
    taxa: Vec<Vec<u32>>,
    /// The rank to map taxa to (e.g., "species", "genus", "family")
    rank: String,
}

#[derive(Serialize)]
pub struct RankMappingResult {
    /// The mapped taxa at the specified rank
    mapped_taxa: Vec<Vec<u32>>,
}

/// Maps taxa to a specific taxonomic rank with caching for duplicate taxa.
/// Uses a HashMap to cache lineage lookups, which is more efficient when there are many duplicates.
async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { taxa, rank }: Parameters,
) -> Result<RankMappingResult, ApiError> {
    let rank_lowercase = rank.to_lowercase();
    let rank_idx = LineageStore::rank_to_idx(&rank_lowercase)
        .ok_or_else(|| ApiError::UnknownRankError(format!("Invalid rank: {}", rank)))?;

    let lineage_store = datastore.lineage_store();

    // Build a cache of taxon_id -> taxon_id_at_rank mappings
    let mut cache: HashMap<u32, Option<u32>> = HashMap::new();

    let mapped_taxa: Vec<Vec<u32>> = taxa
        .iter()
        .map(|taxa_vec| {
            taxa_vec
                .iter()
                .filter_map(|taxon_id| {
                    let mapped_taxon = cache.entry(*taxon_id).or_insert_with(|| {
                        let lineage = get_lineage_array(*taxon_id, LineageVersion::V2, lineage_store);
                        lineage
                            .get(rank_idx)
                            .and_then(|taxon| *taxon)
                            .map(|taxon_id| taxon_id as u32)
                    });

                    *mapped_taxon
                })
                .unique()
                .collect()
        })
        .collect();

    Ok(RankMappingResult {
        mapped_taxa,
    })
}

// Default handler without cache
generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<RankMappingResult>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);

