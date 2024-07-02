use axum::{extract::State, Json};
use database::get_accessions_map;
use serde::{Deserialize, Serialize};

use crate::{controllers::api::{default_equate_il, default_extra}, AppState};

use super::generate_handlers;

#[derive(Deserialize)]
pub struct Parameters {
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ProtInformation {
    Default {
        peptide: String,
        uniprot_id: String,
        protein_name: String,
        taxon_id: u32,
        protein: String,
    },
    Extra {
        peptide: String,
        uniprot_id: String,
        protein_name: String,
        taxon_id: u32,
        taxon_name: String,
        protein: String,
        ec_references: String,
        go_references: String,
        interpro_references: String
    }
}

generate_handlers!(
    async fn handler(
        State(AppState { index, datastore, database }): State<AppState>,
        Parameters { input, equate_il, extra } => Parameters
    ) -> Json<Vec<ProtInformation>> {
        let connection = database.get().await.unwrap();

        let result = index.analyse(&input, equate_il).result;

        let accession_numbers: Vec<String> = result
            .iter()
            .flat_map(|item| item.uniprot_accession_numbers.clone())
            .collect();

        let accessions_map = connection.interact(move |conn| {
            get_accessions_map(conn, &accession_numbers).unwrap()
        }).await.unwrap();

        let taxon_store = datastore.taxon_store();
        
        Json(result.into_iter().map(|item| {
            item.uniprot_accession_numbers.into_iter().map(|accession| {
                let uniprot_entry = accessions_map.get(&accession).unwrap();

                if extra {
                    let (taxon_name, _) = taxon_store.get(uniprot_entry.taxon_id).unwrap();

                    let fa: Vec<&str> = uniprot_entry.fa.split(';').collect();
                    let ec_references = fa.iter().filter(|key| key.starts_with("EC:")).map(ToString::to_string).collect::<Vec<String>>().join(" ");
                    let go_references = fa.iter().filter(|key| key.starts_with("GO:")).map(ToString::to_string).collect::<Vec<String>>().join(" ");
                    let interpro_references = fa.iter().filter(|key| key.starts_with("IPR:")).map(|k| k[4..].to_string()).collect::<Vec<String>>().join(" ");

                    ProtInformation::Extra {
                        peptide: item.sequence.clone(),
                        uniprot_id: accession.clone(),
                        protein_name: uniprot_entry.name.clone(),
                        taxon_id: uniprot_entry.taxon_id,
                        taxon_name: taxon_name.clone(),
                        protein: uniprot_entry.protein.clone(),
                        ec_references,
                        go_references,
                        interpro_references
                    }
                } else {
                    ProtInformation::Default {
                        peptide: item.sequence.clone(),
                        uniprot_id: accession.clone(),
                        protein_name: uniprot_entry.name.clone(),
                        taxon_id: uniprot_entry.taxon_id,
                        protein: uniprot_entry.protein.clone()
                    }
                }
            }).collect::<Vec<ProtInformation>>()
        }).flatten().collect())
    }
);
