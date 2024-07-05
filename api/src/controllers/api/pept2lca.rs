use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_equate_il, default_extra, default_names}, helpers::{lca_helper::calculate_lca, lineage_helper::{get_lineage, get_lineage_with_names, Lineage, LineageVersion::{self, *}}}, AppState};

use crate::controllers::generate_json_handlers;

#[derive(Deserialize)]
pub struct Parameters {
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

generate_json_handlers! (
    [ V1, V2 ]
    async fn handler(
        State(AppState { index, datastore, .. }) => State<AppState>,
        Parameters { input, equate_il, extra, names } => Parameters,
        version: LineageVersion
    ) -> Vec<LcaInformation> {
        let result = index.analyse(&input, equate_il).result;
        
        let taxon_store = datastore.taxon_store();
        let lineage_store = datastore.lineage_store();

        result.into_iter().filter_map(|item| {
            let lca = calculate_lca(item.taxa.iter().map(|&taxon_id| taxon_id as u32).collect(), version, lineage_store);

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
        }).collect::<Vec<LcaInformation>>()
    }
);
