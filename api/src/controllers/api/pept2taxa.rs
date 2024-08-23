use std::collections::HashSet;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_equate_il, default_extra, default_names},
        generate_handlers
    },
    helpers::lineage_helper::{
        get_lineage, get_lineage_with_names, Lineage,
        LineageVersion::{self, *}
    },
    AppState
};
use crate::helpers::sanitize_peptides;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_names")]
    names: bool
}

#[derive(Serialize)]
pub struct TaxaInformation {
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

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra, names }: Parameters,
    version: LineageVersion
) -> Result<Vec<TaxaInformation>, ()> {
    let input = sanitize_peptides(input);
    let result = index.analyse(&input, equate_il, None);

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Ok(result
        .into_iter()
        .flat_map(|item| {
            item.proteins.iter().map(|protein| protein.taxon).collect::<HashSet<u32>>().into_iter().filter_map(
                move |taxon| {
                    let (name, rank, _) = taxon_store.get(taxon)?;
                    let lineage = match (extra, names) {
                        (true, true) => get_lineage_with_names(taxon, version, lineage_store, taxon_store),
                        (true, false) => get_lineage(taxon, version, lineage_store),
                        (false, _) => None
                    };

                    Some(TaxaInformation {
                        peptide: item.sequence.clone(),
                        taxon: Taxon {
                            taxon_id: taxon,
                            taxon_name: name.to_string(),
                            taxon_rank: rank.clone().into()
                        },
                        lineage
                    })
                }
            )
        })
        .collect())
}

generate_handlers! (
    [ V1, V2 ]
    async fn json_handler(
        state => State<AppState>,
        params => Parameters,
        version: LineageVersion
    ) -> Result<Json<Vec<TaxaInformation>>, ()> {
        Ok(Json(handler(state, params, version).await?))
    }
);
