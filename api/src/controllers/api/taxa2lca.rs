use std::convert::Infallible;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{Either, default_extra, default_names, default_validate_taxa},
        generate_handlers,
        request::Flag
    },
    helpers::{
        lca_helper::calculate_lca,
        lineage_helper::{LineageResponse, get_lineage, get_lineage_with_names}
    }
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<Either<u32, String>>,
    #[serde(default = "default_extra")]
    extra: Flag,
    #[serde(default = "default_names")]
    names: Flag,
    #[serde(default = "default_validate_taxa")]
    validate_taxa: Flag
}

#[derive(Serialize)]
pub struct LcaInformation {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    taxon: Option<Taxon>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<LineageResponse>
}

#[derive(Serialize)]
pub struct Taxon {
    taxon_id: u32,
    taxon_name: String,
    taxon_rank: String
}

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters {
        input,
        extra: Flag(extra),
        names: Flag(names),
        validate_taxa: Flag(validate_taxa)
    }: Parameters
) -> Result<LcaInformation, Infallible> {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let casted_input: Vec<u32> = input.iter().map(|v| v.into()).collect();

    // Calculate the LCA of all taxa
    let lca: i32 = calculate_lca(casted_input, taxon_store, lineage_store, validate_taxa);

    if let Some((taxon_name, taxon_rank, _)) = taxon_store.get(lca as u32) {
        // Calculate the lineage of the LCA
        let lineage = match (extra, names) {
            (true, true) => get_lineage_with_names(lca as u32, lineage_store, taxon_store),
            (true, false) => get_lineage(lca as u32, lineage_store),
            (false, _) => None
        };

        return Ok(LcaInformation {
            taxon: Some(Taxon {
                taxon_id: lca as u32,
                taxon_name: taxon_name.to_string(),
                taxon_rank: taxon_rank.to_string()
            }),
            lineage
        });
    }

    Ok(LcaInformation { taxon: None, lineage: None })
}

generate_handlers! (
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<LcaInformation>, Infallible> {
        Ok(Json(handler(state, params).await?))
    }
);
