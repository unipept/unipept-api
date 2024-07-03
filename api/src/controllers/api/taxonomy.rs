use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_extra, default_names}, helpers::lineage_helper::{get_lineage, get_lineage_with_names, Lineage, LineageVersion::{self, *}}, AppState};

use crate::controllers::generate_handlers;

#[derive(Deserialize)]
pub struct Parameters {
    input: Vec<u32>,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_names")]
    names: bool
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

generate_handlers!(
    [ V1, V2 ]
    async fn handler(
        State(AppState { datastore, .. }): State<AppState>,
        Parameters { input, extra, names } => Parameters,
        version: LineageVersion
    ) -> Json<Vec<TaxaInformation>> {
        if input.is_empty() {
            return Json(Vec::new());
        }
    
        let taxon_store = datastore.taxon_store();
        let lineage_store = datastore.lineage_store();
    
        Json(input
            .into_iter()
            .filter_map(|taxon_id| {
                let (name, rank) = taxon_store.get(taxon_id)?;
                let lineage = match (extra, names) {
                    (true, true)  => get_lineage_with_names(taxon_id, version, lineage_store, taxon_store),
                    (true, false) => get_lineage(taxon_id, version, lineage_store),
                    (false, _)    => None    
                };
                
                Some(TaxaInformation {
                    taxon: Taxon {
                        taxon_id: taxon_id,
                        taxon_name: name.to_string(),
                        taxon_rank: rank.clone().into()
                    },
                    lineage
                })
            })
            .collect()
        )
    }
);
