use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_extra, default_names},
        generate_handlers
    },
    helpers::{
        lca_helper::calculate_lca,
        lineage_helper::{
            get_lineage, get_lineage_with_names, Lineage,
            LineageVersion::{self, *}
        }
    },
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    input: Vec<u32>,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_names")]
    names: bool
}

#[derive(Serialize)]
pub struct LcaInformation {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    taxon: Option<Taxon>,
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
    Parameters { input, extra, names }: Parameters,
    version: LineageVersion
) -> Result<LcaInformation, ()> {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    // Calculate the LCA of all taxa
    let lca: i32 = calculate_lca(input, version, taxon_store, lineage_store);

    if let Some((taxon_name, taxon_rank, _)) = taxon_store.get(lca as u32) {
        // Calculate the lineage of the LCA
        let lineage = match (extra, names) {
            (true, true) => get_lineage_with_names(lca as u32, version, lineage_store, taxon_store),
            (true, false) => get_lineage(lca as u32, version, lineage_store),
            (false, _) => None
        };

        return Ok(LcaInformation {
            taxon: Some(Taxon {
                taxon_id: lca as u32,
                taxon_name: taxon_name.to_string(),
                taxon_rank: taxon_rank.clone().into()
            }),
            lineage
        });
    }

    Ok(LcaInformation { taxon: None, lineage: None })
}

generate_handlers! (
    [ V1, V2 ]
    async fn json_handler(
        state => State<AppState>,
        params => Parameters,
        version: LineageVersion
    ) -> Result<Json<LcaInformation>, ()> {
        Ok(Json(handler(state, params, version).await?))
    }
);
