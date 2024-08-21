use std::collections::HashSet;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use datastore::LineageStore;
use crate::{
    controllers::{
        api::{default_extra, default_names, default_descendants, default_descendants_rank},
        generate_handlers
    },
    helpers::lineage_helper::{
        get_lineage, get_lineage_with_names, Lineage,
        LineageVersion::{self, *}
    },
    AppState
};

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
    #[serde(default = "default_descendants_rank")]
    descendants_rank: String
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

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { input, extra, names, descendants, descendants_rank }: Parameters,
    version: LineageVersion
) -> Result<Vec<TaxaInformation>, ()> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Ok(input
        .into_iter()
        .filter_map(|taxon_id| {
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
                    let descendants_rank: String = descendants_rank.to_string().to_lowercase();

                    // Check if the provided rank is valid
                    if LineageStore::rank_to_idx(descendants_rank.as_str()).is_none() {
                        // TODO update to return the proper HTTP status code and an appropriate error message.
                        panic!("Cannot retrieve descendants for unknown rank.")
                    }

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
                        .for_each(|id| { children_id_set.insert(id.abs() as u32); });

                    Some(Vec::from_iter(children_id_set))
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
        .collect())
}

generate_handlers!(
    [ V1, V2 ]
    async fn json_handler(
        state => State<AppState>,
        params => Parameters,
        version: LineageVersion
    ) -> Result<Json<Vec<TaxaInformation>>, ()> {
        Ok(Json(handler(state, params, version).await?))
    }
);
