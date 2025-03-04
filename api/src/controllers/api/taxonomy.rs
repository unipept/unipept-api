use std::collections::HashSet;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use datastore::{LineageRank, LineageStore};
use crate::{
    controllers::{
        api::{default_extra, default_names, default_descendants, default_descendants_ranks},
        generate_handlers
    },
    helpers::lineage_helper::{
        get_lineage,
        get_lineage_with_names,
        get_empty_lineage,
        get_empty_lineage_with_names,
        Lineage,
        LineageVersion::{self, *}
    },
    AppState
};
use crate::errors::ApiError;
use crate::errors::ApiError::UnknownRankError;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<u32>,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_names")]
    names: bool,
    #[serde(default = "default_descendants")]
    descendants: bool,
    #[serde(default = "default_descendants_ranks")]
    descendants_ranks: Vec<String>
}

#[derive(Serialize)]
pub struct TaxaInformation {
    #[serde(flatten)]
    taxon: Taxon,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<Lineage>,
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
///     taxonomic information from the database.
fn get_children_at_rank(
    taxon_id: u32,
    rank: LineageRank,
    descendants_ranks: String,
    lineage_store: &LineageStore
) -> Option<HashSet<u32>> {
    let descendants_rank: String = descendants_ranks.to_string().to_lowercase();

    let lineages_at_rank = lineage_store.get_lineages_at_rank(
        rank.to_string().to_lowercase().as_str(),
        taxon_id
    );

    let mut children_id_set = HashSet::new();

    lineages_at_rank?
        .iter()
        .filter_map(
            |lin| {
                lin.get_taxon_id_at_rank(descendants_rank.as_str())
            }
        )
        .for_each(|id| { children_id_set.insert(id.unsigned_abs()); });

    Some(children_id_set)
}

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { input, extra, names, descendants, descendants_ranks }: Parameters,
    version: LineageVersion
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
                return Err(UnknownRankError(String::from("An unknown rank has been passed for the `descendant_rank` parameter.")))
            }
        }
    }

   Ok(
       input
        .into_iter()
        .filter_map(|taxon_id| {
            // The root taxon is a special case.
            if taxon_id == 1 {
                let mut children: Option<Vec<u32>> = None;

                // If descendants is true, we need to get all the taxa at the requested level and
                // report those as children of the root.
                if descendants {
                    children = Some(lineage_store.get_all_taxon_ids_at_rank("superkingdom")?
                        .iter()
                        .flat_map(|sk_taxon| {
                            descendants_ranks
                                .iter()
                                .cloned()
                                .filter_map(|desc_rank| get_children_at_rank(*sk_taxon, LineageRank::Superkingdom, desc_rank, lineage_store))
                                .flat_map(|set| set.into_iter())
                                .collect::<Vec<u32>>()
                        })
                        .collect());
                }

                let lineage: Option<Lineage> = match (extra, names) {
                    (true, true) => get_empty_lineage_with_names(version),
                    (true, false) => get_empty_lineage(version),
                    (false, _) => None
                };

                return Some(TaxaInformation {
                    taxon: Taxon {
                        taxon_id,
                        taxon_name: String::from("root"),
                        taxon_rank: String::from("no rank")
                    },
                    lineage,
                    descendants: children
                });
            }

            let (name, rank, _) = taxon_store.get(taxon_id)?;
            let lineage = match (extra, names) {
                (true, true) => get_lineage_with_names(taxon_id, version, lineage_store, taxon_store),
                (true, false) => get_lineage(taxon_id, version, lineage_store),
                (false, _) => None
            };

            // If the user would like to get all the descendants of the given taxon, we'll try to
            // retrieve these here. These descendants are just a list of taxon IDs.
            let children: Option<Vec<u32>> = match descendants {
                true => {
                    // Retrieve information for all descendant ranks that are provided to this
                    // function
                    let mut child_vector: Vec<u32> = Vec::new();

                    for desc_rank in descendants_ranks.clone() {
                        let items = get_children_at_rank(
                            taxon_id,
                            rank.clone(),
                            desc_rank,
                            lineage_store
                        );

                        if let Some(values) = items {
                            child_vector.extend(values.into_iter())
                        }
                    }

                    Some(child_vector)
                },
                false => None
            };

            Some(TaxaInformation {
                taxon: Taxon {
                    taxon_id,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.clone().into()
                },
                lineage,
                descendants: children
            })
        })
        .collect()
   )
}

generate_handlers!(
    [ V1, V2 ]
    async fn json_handler(
        state => State<AppState>,
        params => Parameters,
        version: LineageVersion
    ) -> Result<Json<Vec<TaxaInformation>>, ApiError> {
        Ok(Json(handler(state, params, version).await?))
    }
);
