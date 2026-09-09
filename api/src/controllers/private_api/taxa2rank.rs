use std::collections::HashMap;

use axum::{Json, extract::State};
use datastore::LineageStore;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{AppState, controllers::generate_handlers, errors::ApiError, helpers::lineage_helper::get_lineage_array};

#[derive(Deserialize)]
pub struct Parameters {
    /// Vector of taxa vectors, one per peptide
    taxa: Vec<Vec<u32>>,
    /// The rank to map taxa to, as the lineage columns spell it: `species`, `genus`,
    /// `species_group` with an underscore rather than a space.
    rank: String
}

#[derive(Serialize)]
pub struct RankMappingResult {
    /// The mapped taxa at the specified rank
    mapped_taxa: Vec<Vec<u32>>
}

/// Maps taxa to a specific taxonomic rank with caching for duplicate taxa.
async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { taxa, rank }: Parameters
) -> Result<RankMappingResult, ApiError> {
    // Read with its case intact. A rank names a lineage column here rather than matching text a
    // user typed, so `GENUS` is not `genus`, exactly as for `descendants_ranks` on
    // `/api/v2/taxonomy`. Which spellings `rank_to_idx` accepts is its own business.
    let rank_idx = LineageStore::rank_to_idx(&rank)
        .ok_or_else(|| ApiError::UnknownRankError(format!("Invalid rank: {}", rank)))?;

    let lineage_store = datastore.lineage_store();

    let mut cache: HashMap<u32, Option<u32>> = HashMap::new();

    let mapped_taxa: Vec<Vec<u32>> = taxa
        .iter()
        .map(|taxa_vec| {
            taxa_vec
                .iter()
                .filter_map(|taxon_id| {
                    let mapped_taxon = cache.entry(*taxon_id).or_insert_with(|| {
                        let lineage = get_lineage_array(*taxon_id, lineage_store);
                        lineage.get(rank_idx).and_then(|taxon| *taxon).map(|taxon_id| taxon_id as u32)
                    });

                    *mapped_taxon
                })
                .unique()
                .collect()
        })
        .collect();

    Ok(RankMappingResult { mapped_taxa })
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<RankMappingResult>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
