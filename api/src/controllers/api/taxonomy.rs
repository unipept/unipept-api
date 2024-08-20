use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

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
    lineage: Option<Lineage>
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

            match (lineage) {
                Some(ref l) => {
                    println!("{:?}", l);
                },
                None => {
                    println!("No lineage found...");
                }
            }


            // If the user would like to get all the descendants of the given taxon, we'll try to
            // retrieve these here. These descendants are just a list of taxon IDs.
            let mut child_ids: Option<Vec<u32>> = None;
            if (descendants) {
                // First, we check at which rank ID the provided input taxon is situated


                let mut child_vec: Vec<u32> = Vec::new();
                let k = 7;
            }


            Some(TaxaInformation {
                taxon: Taxon {
                    taxon_id,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.clone().into()
                },
                lineage
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
