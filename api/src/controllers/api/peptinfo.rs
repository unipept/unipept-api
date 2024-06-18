use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_domains, default_equate_il, default_extra, default_names}, helpers::{ec_helper::{ec_numbers, EcNumber}, go_helper::{go_terms, GoTerms}, interpro_helper::{interpro_entries, InterproEntries}, lineage_helper::{lineages, Lineage, LineageVersion}}, AppState};

use super::Query;

#[derive(Deserialize)]
pub struct QueryParams {
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_domains")]
    domains: bool,
    #[serde(default = "default_names")]
    names: bool
}

#[derive(Serialize)]
pub struct PeptInformation {
    peptide: String,
    total_protein_count: usize,
    ec: Vec<EcNumber>,
    go: GoTerms,
    ipr: InterproEntries,
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
) -> Json<Vec<PeptInformation>> {
    handler(state, query, LineageVersion::V1)
}

pub async fn handler_v2(
    state: State<AppState>,
    query: Query<QueryParams>
) -> Json<Vec<PeptInformation>> {
    handler(state, query, LineageVersion::V2)
}

fn handler(
    State(AppState { index, datastore }): State<AppState>,
    Query(QueryParams { input, equate_il, extra, domains, names }): Query<QueryParams>,
    version: LineageVersion
) -> Json<Vec<PeptInformation>> {
    let result = index.analyse(&input, equate_il).result;
    
    let ec_store = datastore.ec_store();
    let go_store = datastore.go_store();
    let interpro_store = datastore.interpro_store();
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Json(result.into_iter().filter_map(|item| {
        let fa = item.fa?;
        
        let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
        let ecs = ec_numbers(&fa.data, ec_store, extra);
        let gos = go_terms(&fa.data, go_store, extra, domains);
        let iprs = interpro_entries(&fa.data, interpro_store, extra, domains);

        let lca = item.lca?;
        let (name, rank) = taxon_store.get(lca as u32)?;
        let lineage = if extra {
            lineages(lca as u32, names, lineage_store, taxon_store, version)
        } else {
            None
        };

        Some(PeptInformation {
            peptide: item.sequence,
            total_protein_count,
            ec: ecs,
            go: gos,
            ipr: iprs,
            taxon: Taxon {
                taxon_id: lca as u32,
                taxon_name: name.to_string(),
                taxon_rank: rank.clone().into()
            },
            lineage
        })
    }).collect())
}
