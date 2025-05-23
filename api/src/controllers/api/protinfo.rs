use std::collections::HashSet;
use axum::{extract::State, Json};
use database::get_accessions;
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_domains, default_extra, default_names},
        generate_handlers
    },
    errors::ApiError,
    helpers::{
        ec_helper::{ec_numbers_from_list, EcNumber},
        go_helper::{go_terms_from_list, GoTerms},
        interpro_helper::{interpro_entries_from_list, InterproEntries},
        lineage_helper::{
            get_lineage, get_lineage_with_names, Lineage,
            LineageVersion::{self, *}
        }
    },
    AppState
};
use crate::helpers::sanitize_proteins;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_domains")]
    domains: bool,
    #[serde(default = "default_names")]
    names: bool
}

#[derive(Serialize)]
pub struct ProtInformation {
    protein: String,
    name: String,
    #[serde(flatten)]
    taxon: Taxon,
    ec: Vec<EcNumber>,
    go: GoTerms,
    ipr: InterproEntries,
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
    State(AppState { datastore, database, .. }): State<AppState>,
    Parameters { input, extra, domains, names }: Parameters,
    version: LineageVersion
) -> Result<Vec<ProtInformation>, ApiError> {
    let input = sanitize_proteins(input);
    let input = HashSet::from_iter(input.into_iter());

    let connection = database.get_conn();

    let entries = get_accessions(connection, &input).await?;

    let ec_store = datastore.ec_store();
    let go_store = datastore.go_store();
    let interpro_store = datastore.interpro_store();
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let fa: Vec<&str> = entry.fa.split(';').collect();
            let ecs = ec_numbers_from_list(&fa, ec_store, extra);
            let gos = go_terms_from_list(&fa, go_store, extra, domains);
            let iprs = interpro_entries_from_list(&fa, interpro_store, extra, domains);

            let (name, rank, _) = taxon_store.get(entry.taxon_id)?;
            let lineage = match (extra, names) {
                (true, true) => get_lineage_with_names(entry.taxon_id, version, lineage_store, taxon_store),
                (true, false) => get_lineage(entry.taxon_id, version, lineage_store),
                (false, _) => None
            };

            Some(ProtInformation {
                protein: entry.uniprot_accession_number,
                name: entry.name,
                taxon: Taxon {
                    taxon_id: entry.taxon_id,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.clone().into()
                },
                ec: ecs,
                go: gos,
                ipr: iprs,
                lineage
            })
        })
        .collect())
}

generate_handlers! (
    [ V2 ]
    async fn json_handler(
        state => State<AppState>,
        params => Parameters,
        version: LineageVersion
    ) -> Result<Json<Vec<ProtInformation>>, ApiError> {
        Ok(Json(handler(state, params, version).await?))
    }
);
