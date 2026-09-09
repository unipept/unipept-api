use std::collections::BTreeSet;

use axum::{Json, extract::State};
use datastore::{LineageStore, TaxonRank};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_descendants, default_descendants_ranks, default_extra, default_names},
        generate_handlers,
        request::Flag
    },
    errors::{ApiError, ApiError::UnknownRankError},
    helpers::lineage_helper::{
        LineageResponse, get_empty_lineage, get_empty_lineage_with_names, get_lineage, get_lineage_with_names
    }
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<u32>,
    #[serde(default = "default_extra")]
    extra: Flag,
    #[serde(default = "default_names")]
    names: Flag,
    #[serde(default = "default_descendants")]
    descendants: Flag,
    /// Rank names, spelled with either separator: `species_group` and `species group` both read.
    /// `LineageStore::rank_to_idx` normalises the separator, and matches the rest exactly.
    #[serde(default = "default_descendants_ranks")]
    descendants_ranks: Vec<String>
}

#[derive(Serialize)]
pub struct TaxaInformation {
    #[serde(flatten)]
    taxon: Taxon,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<LineageResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    descendants: Option<Vec<u32>>
}

#[derive(Serialize)]
pub struct Taxon {
    taxon_id: u32,
    taxon_name: String,
    taxon_rank: String
}

/// Retrieve all child IDs for a specific taxon.
///
/// # Arguments
///
/// * `taxon_id` - ID of the taxon for which all taxon child IDs should be retrieved.
/// * `rank` - The rank of the taxon that was passed using the `taxon_id` parameter
/// * `descendants_rank` - The rank from which the children should be retrieved.
/// * `lineage_store` - A reference to the LineageStore that can be used to retrieve lineages and
///   taxonomic information from the database.
fn get_children_at_rank(
    taxon_id: u32,
    rank: TaxonRank,
    descendants_rank: &str,
    lineage_store: &LineageStore,
    descendant_ids: &mut BTreeSet<u32>
) {
    // Taken as it arrived: `handler` rejects a rank `rank_to_idx` does not know before any of them
    // reaches here. That check reads either separator, a space or an underscore, and is
    // case-sensitive.
    // The column is the same for every lineage below the taxon, and a coarse rank reaches a
    // million of them.
    let Some(column) = LineageStore::rank_to_idx(descendants_rank) else {
        return;
    };

    let Some(lineages_at_rank) = lineage_store.get_lineages_at_rank(rank, taxon_id) else {
        return;
    };

    descendant_ids.extend(lineages_at_rank.iter().filter_map(|lin| lin.get_rank(column)).map(i32::unsigned_abs));
}

/// Adds the descendants of one taxon, over every rank the request named.
///
/// One set for every rank rather than one set each: a request naming two ranks reads one ascending
/// list of taxon ids, not one ascending run per rank.
///
/// The set also bounds what is held. A coarse rank answers with a few dozen ids and reaches them
/// through every lineage below the taxon — a million of them under a large domain — so collecting
/// the ids first and ordering them afterwards would hold every repeat at once.
fn descendants_at_ranks(
    taxon_id: u32,
    rank: TaxonRank,
    descendants_ranks: &[String],
    lineage_store: &LineageStore,
    descendant_ids: &mut BTreeSet<u32>
) {
    for descendants_rank in descendants_ranks {
        get_children_at_rank(taxon_id, rank, descendants_rank, lineage_store, descendant_ids);
    }
}

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters {
        input,
        extra: Flag(extra),
        names: Flag(names),
        descendants: Flag(descendants),
        descendants_ranks
    }: Parameters
) -> Result<Vec<TaxaInformation>, ApiError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    // Check if the provided ranks are actually valid and known
    if descendants {
        for desc_rank in descendants_ranks.clone() {
            if LineageStore::rank_to_idx(desc_rank.as_str()).is_none() {
                return Err(UnknownRankError(String::from(
                    "An unknown rank has been passed for the `descendant_rank` parameter."
                )));
            }
        }
    }

    Ok(input
        .into_iter()
        .filter_map(|taxon_id| {
            // The root taxon is a special case.
            if taxon_id == 1 {
                let mut children: Option<Vec<u32>> = None;

                // If descendants is true, we need to get all the taxa at the requested level and
                // report those as children of the root.
                if descendants {
                    let mut descendant_ids = BTreeSet::new();

                    for top_taxon in lineage_store.get_all_taxon_ids_at_rank(TaxonRank::TOP_RANK)? {
                        descendants_at_ranks(
                            top_taxon,
                            TaxonRank::TOP_RANK,
                            &descendants_ranks,
                            lineage_store,
                            &mut descendant_ids
                        );
                    }

                    children = Some(descendant_ids.into_iter().collect());
                }

                let lineage: Option<LineageResponse> = match (extra, names) {
                    (true, true) => get_empty_lineage_with_names(),
                    (true, false) => get_empty_lineage(),
                    (false, _) => None
                };

                return Some(TaxaInformation {
                    taxon: Taxon {
                        taxon_id,
                        taxon_name: String::from("root"),
                        taxon_rank: TaxonRank::NO_RANK.to_string()
                    },
                    lineage,
                    descendants: children
                });
            }

            let (name, rank, _) = taxon_store.get(taxon_id)?;
            let lineage = match (extra, names) {
                (true, true) => get_lineage_with_names(taxon_id, lineage_store, taxon_store),
                (true, false) => get_lineage(taxon_id, lineage_store),
                (false, _) => None
            };

            // If the user would like to get all the descendants of the given taxon, we'll try to
            // retrieve these here. These descendants are just a list of taxon IDs.
            let children: Option<Vec<u32>> = descendants.then(|| {
                let mut descendant_ids = BTreeSet::new();
                descendants_at_ranks(taxon_id, *rank, &descendants_ranks, lineage_store, &mut descendant_ids);
                descendant_ids.into_iter().collect()
            });

            Some(TaxaInformation {
                taxon: Taxon {
                    taxon_id,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.to_string()
                },
                lineage,
                descendants: children
            })
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<TaxaInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
