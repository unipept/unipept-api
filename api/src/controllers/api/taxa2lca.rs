use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_extra, default_names}, helpers::lineage_helper::{get_lineage, get_lineage_with_names, Lineage, LineageVersion}, AppState};

use super::Query;

#[derive(Deserialize)]
pub struct QueryParams {
    input: Vec<u32>,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_names")]
    names: bool
}

#[derive(Serialize)]
pub struct LcaInformation {
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

pub async fn handler_v1(
    state: State<AppState>,
    query: Query<QueryParams>
) -> Json<LcaInformation> {
    handler(state, query, LineageVersion::V1)
}

pub async fn handler_v2(
    state: State<AppState>,
    query: Query<QueryParams>
) -> Json<LcaInformation> {
    handler(state, query, LineageVersion::V2)
}

fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Query(QueryParams { input: _input, extra, names }): Query<QueryParams>,
    version: LineageVersion
) -> Json<LcaInformation> {    
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    // TODO: calculate the LCA
    let lca: u32 = 1;

    let (name, rank) = taxon_store.get(lca).unwrap(); // TODO: We should not just call unwrap here
    let lineage = match (extra, names) {
        (true, true)  => get_lineage_with_names(lca as u32, version, lineage_store, taxon_store),
        (true, false) => get_lineage(lca as u32, version, lineage_store),
        (false, _)    => None    
    };

    Json(LcaInformation {
        taxon: Taxon {
            taxon_id: lca,
            taxon_name: name.to_string(),
            taxon_rank: rank.clone().into()
        },
        lineage
    })
}
