use axum::{
    extract::State,
    Json
};
use database::get_accessions;
use serde::{
    Deserialize,
    Serialize
};

use crate::{
    controllers::{
        api::{
            default_domains,
            default_extra,
            default_names
        },
        generate_json_handlers
    },
    helpers::{
        ec_helper::{
            ec_numbers_from_list,
            EcNumber
        },
        go_helper::{
            go_terms_from_list,
            GoTerms
        },
        interpro_helper::{
            interpro_entries_from_list,
            InterproEntries
        },
        lineage_helper::{
            get_lineage,
            get_lineage_with_names,
            Lineage,
            LineageVersion::{
                self,
                *
            }
        }
    },
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    input:   Vec<String>,
    #[serde(default = "default_extra")]
    extra:   bool,
    #[serde(default = "default_domains")]
    domains: bool,
    #[serde(default = "default_names")]
    names:   bool
}

#[derive(Serialize)]
pub struct ProtInformation {
    protein: String,
    #[serde(flatten)]
    taxon:   Taxon,
    ec:      Vec<EcNumber>,
    go:      GoTerms,
    ipr:     InterproEntries,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<Lineage>
}

#[derive(Serialize)]
pub struct Taxon {
    taxon_id:   u32,
    taxon_name: String,
    taxon_rank: String
}

generate_json_handlers!(
    [ V1, V2 ]
    async fn handler(
        State(AppState { datastore, database, .. }) => State<AppState>,
        Parameters { input, extra, domains, names } => Parameters,
        version: LineageVersion
    ) -> Vec<ProtInformation> {
        let connection = database.get().await.unwrap();

        let entries = connection.interact(move |conn|
            get_accessions(conn, &input).unwrap()
        ).await.unwrap();

        let ec_store = datastore.ec_store();
        let go_store = datastore.go_store();
        let interpro_store = datastore.interpro_store();
        let taxon_store = datastore.taxon_store();
        let lineage_store = datastore.lineage_store();

        entries.into_iter().map(|entry| {
            let fa: Vec<&str> = entry.fa.split(';').collect();
            let ecs = ec_numbers_from_list(&fa, ec_store, extra);
            let gos = go_terms_from_list(&fa, go_store, extra, domains);
            let iprs = interpro_entries_from_list(&fa, interpro_store, extra, domains);

            let (name, rank) = taxon_store.get(entry.taxon_id).unwrap();
            let lineage = match (extra, names) {
                (true, true)  => get_lineage_with_names(entry.taxon_id, version, lineage_store, taxon_store),
                (true, false) => get_lineage(entry.taxon_id, version, lineage_store),
                (false, _)    => None
            };

            ProtInformation {
                protein: entry.uniprot_accession_number,
                taxon: Taxon {
                    taxon_id: entry.taxon_id,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.clone().into()
                },
                ec: ecs,
                go: gos,
                ipr: iprs,
                lineage
            }
        }).collect()
    }
);
