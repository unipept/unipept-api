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
    children: Option<Vec<u32>>
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
                (true) => {
                    let lineages_at_rank = lineage_store.get_lineages_at_rank(rank.to_string().to_lowercase().as_str(), taxon_id);
                    match lineages_at_rank {
                        Some(lins) => {
                            let mut children_at_rank: HashSet<u32> = HashSet::new();
                            for lin in lins {
                                let taxon_id_at_rank = lin.get_taxon_id_at_rank(descendants_rank.to_lowercase().as_str());
                                match taxon_id_at_rank {
                                    Some(tax) => { children_at_rank.insert(tax.abs() as u32); },
                                    None => {}
                                }
                            }
                            Some(Vec::from_iter(children_at_rank))
                        },
                        None => None
                    }
                },
                (false) => None
            };

            println!("{:?}", children);

            Some(TaxaInformation {
                taxon: Taxon {
                    taxon_id,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.clone().into()
                },
                lineage,
                children
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
