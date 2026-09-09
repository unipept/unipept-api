use axum::{Json, extract::State};
use database::get_accessions;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_domains, default_extra, default_names},
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{
        ec_helper::{EcNumber, ec_numbers_from_list},
        go_helper::{GoTerms, go_terms_from_list},
        interpro_helper::{InterproEntries, interpro_entries_from_list},
        lineage_helper::{
            Lineage,
            LineageVersion::{self, *},
            get_lineage, get_lineage_with_names
        },
        sanitize_proteins
    }
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_extra")]
    extra: Flag,
    #[serde(default = "default_domains")]
    domains: Flag,
    #[serde(default = "default_names")]
    names: Flag
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
    Parameters {
        input,
        extra: Flag(extra),
        domains: Flag(domains),
        names: Flag(names)
    }: Parameters,
    version: LineageVersion
) -> Result<Vec<ProtInformation>, ApiError> {
    // Each accession once, in first-appearance order, which is the order the answer takes.
    let input: Vec<String> = sanitize_proteins(input).into_iter().unique().collect();

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
                    taxon_rank: (*rank).into()
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
