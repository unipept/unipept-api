use std::collections::{HashMap, HashSet};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use datastore::{LineageRank, LineageStore};
use database::get_protein_counts_by_taxon_ids;

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    helpers::lineage_helper::{get_lineage_array, LineageVersion},
    AppState
};

fn default_report_protein_count() -> bool {
    false
}

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    taxids: Vec<i32>,
    #[serde(default = "default_report_protein_count")]
    report_protein_count: bool
}

#[derive(Serialize)]
pub struct Taxon {
    id: u32,
    name: String,
    rank: String,
    lineage: Vec<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protein_count: Option<u32>
}

/// Resolves a taxon to the set of species/strain-level taxon IDs whose protein counts should be
/// summed to produce the total protein count for this taxon.
fn collect_protein_taxon_ids(
    taxon_id: u32,
    rank: &LineageRank,
    lineage_store: &LineageStore
) -> HashSet<u32> {
    let rank_str = String::from(rank.clone());

    match rank {
        LineageRank::Strain => {
            let mut ids = HashSet::new();
            ids.insert(taxon_id);
            ids
        }
        LineageRank::Species => {
            let mut ids = HashSet::new();
            ids.insert(taxon_id);
            if let Some(lineages) = lineage_store.get_lineages_at_rank(&rank_str, taxon_id) {
                for lin in lineages {
                    if let Some(strain_id) = lin.strain {
                        ids.insert(strain_id.unsigned_abs());
                    }
                }
            }
            ids
        }
        _ => {
            let mut ids = HashSet::new();
            if let Some(lineages) = lineage_store.get_lineages_at_rank(&rank_str, taxon_id) {
                for lin in lineages {
                    if let Some(species_id) = lin.species {
                        ids.insert(species_id.unsigned_abs());
                    }
                    if let Some(strain_id) = lin.strain {
                        ids.insert(strain_id.unsigned_abs());
                    }
                }
            }
            ids
        }
    }
}

async fn handler(
    State(AppState { datastore, database, .. }): State<AppState>,
    Parameters { taxids, report_protein_count }: Parameters
) -> Result<Vec<Taxon>, ApiError> {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let protein_counts: HashMap<u32, u32> = if report_protein_count {
        let mut taxon_to_leaf_ids: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut all_leaf_ids: HashSet<u32> = HashSet::new();

        for &taxon_id in &taxids {
            if taxon_id <= 0 { continue; }
            let id = taxon_id as u32;
            if let Some((_, rank, _)) = taxon_store.get(id) {
                let leaf_ids = collect_protein_taxon_ids(id, rank, lineage_store);
                all_leaf_ids.extend(&leaf_ids);
                taxon_to_leaf_ids.insert(id, leaf_ids);
            }
        }

        let all_leaf_ids_vec: Vec<i32> = all_leaf_ids.into_iter().map(|id| id as i32).collect();
        let leaf_counts = get_protein_counts_by_taxon_ids(database.get_conn(), &all_leaf_ids_vec).await?;

        taxon_to_leaf_ids
            .into_iter()
            .map(|(orig_id, leaf_ids)| {
                let total: u32 = leaf_ids.iter()
                    .map(|&lid| *leaf_counts.get(&lid).unwrap_or(&0))
                    .sum();
                (orig_id, total)
            })
            .collect()
    } else {
        HashMap::new()
    };

    Ok(taxids
        .into_iter()
        .filter(|&taxon_id| taxon_id > 0)
        .filter_map(|taxon_id| {
            let (name, rank, _) = taxon_store.get(taxon_id as u32)?;
            let lineage = get_lineage_array(taxon_id as u32, LineageVersion::V2, lineage_store);
            let protein_count = if report_protein_count {
                Some(*protein_counts.get(&(taxon_id as u32)).unwrap_or(&0))
            } else {
                None
            };

            Some(Taxon {
                id: taxon_id as u32,
                name: name.clone(),
                rank: rank.clone().into(),
                lineage,
                protein_count
            })
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<Taxon>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
