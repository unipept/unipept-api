use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_equate_il, default_extra, default_names}, helpers::lineage_helper::{get_lineage, get_lineage_with_names, Lineage, LineageVersion}, AppState};

use super::Query;

#[derive(Deserialize)]
pub struct QueryParams {
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_names")]
    names: bool
}

#[derive(Serialize)]
pub struct LcaInformation {
    peptide: String,
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
) -> Json<Vec<LcaInformation>> {
    handler(state, query, LineageVersion::V1)
}

pub async fn handler_v2(
    state: State<AppState>,
    query: Query<QueryParams>
) -> Json<Vec<LcaInformation>> {
    handler(state, query, LineageVersion::V2)
}

fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra, names }): Query<QueryParams>,
    version: LineageVersion
) -> Json<Vec<LcaInformation>> {
    let result = index.analyse(&input, equate_il).result;
    
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Json(result.into_iter().filter_map(|item| {
        let lca = item.lca?;
        let (name, rank) = taxon_store.get(lca as u32)?;
        let lineage = match (extra, names) {
            (true, true)  => get_lineage_with_names(lca as u32, version, lineage_store, taxon_store),
            (true, false) => get_lineage(lca as u32, version, lineage_store),
            (false, _)    => None    
        };

        Some(LcaInformation {
            peptide: item.sequence,
            taxon: Taxon {
                taxon_id: lca as u32,
                taxon_name: name.to_string(),
                taxon_rank: rank.clone().into()
            },
            lineage
        })
    }).collect())
}
