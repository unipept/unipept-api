use std::convert::Infallible;

use axum::{Json, extract::State};
use itertools::Itertools;
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
        lineage_helper::{LineageResponse, Taxon, lineage_for}
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

    let casted_input: Vec<u32> = input.iter().map(|v| v.into()).unique().collect();

    let lca: i32 = calculate_lca(casted_input, taxon_store, lineage_store, validate_taxa);

    if let Some(taxon) = Taxon::new(lca as u32, taxon_store) {
        let lineage = lineage_for(lca as u32, extra, names, lineage_store, taxon_store);

        return Ok(LcaInformation { taxon: Some(taxon), lineage });
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
