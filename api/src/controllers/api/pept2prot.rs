use std::collections::HashSet;
use axum::{extract::State, Json};
use database::get_accessions_map;
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_equate_il, default_extra, default_tryptic, default_cutoff},
        generate_handlers
    },
    errors::ApiError,
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
    #[serde(default = "default_tryptic")]
    tryptic: bool,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ProtInformation {
    Default {
        peptide: String,
        uniprot_id: String,
        protein_name: String,
        taxon_id: u32,
        protein: String
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

async fn handler(
    State(AppState { index, datastore, database }): State<AppState>,
    Parameters { input, equate_il, extra, tryptic, cutoff }: Parameters
) -> Result<Vec<ProtInformation>, ApiError> {
    let input = sanitize_peptides(input);

    let connection = database.get_conn();

    let result = index.analyse(&input, equate_il, tryptic, Some(cutoff));

    let accession_numbers: HashSet<String> = result
        .iter()
        .flat_map(|item| item.proteins.iter().map(|protein| protein.uniprot_accession.clone()))
        .collect();

    let accessions_map = get_accessions_map(connection, &accession_numbers)
        .await
        .map_err(|e| {
            println!("Error occurred: {:?}", e);
            e
        })?;

    let taxon_store = datastore.taxon_store();

    Ok(result
        .into_iter()
        .flat_map(|item| {
            item.proteins
                .into_iter()
                .filter_map(|protein| {
                    let uniprot_entry = accessions_map.get(&protein.uniprot_accession)?;

                    if extra {
                        let taxon_name = taxon_store.get_name(uniprot_entry.taxon_id)?;

                        let fa: Vec<&str> = uniprot_entry.fa.split(';').collect();
                        let ec_references = fa
                            .iter()
                            .filter(|key| key.starts_with("EC:"))
                            .map(ToString::to_string)
                            .collect::<Vec<String>>()
                            .join(" ");
                        let go_references = fa
                            .iter()
                            .filter(|key| key.starts_with("GO:"))
                            .map(ToString::to_string)
                            .collect::<Vec<String>>()
                            .join(" ");
                        let interpro_references = fa
                            .iter()
                            .filter(|key| key.starts_with("IPR:"))
                            .map(|k| k[4..].to_string())
                            .collect::<Vec<String>>()
                            .join(" ");

                        Some(ProtInformation::Extra {
                            peptide: item.sequence.clone(),
                            uniprot_id: protein.uniprot_accession.clone(),
                            protein_name: uniprot_entry.name.clone(),
                            taxon_id: uniprot_entry.taxon_id,
                            taxon_name: taxon_name.clone(),
                            protein: uniprot_entry.protein.clone(),
                            ec_references,
                            go_references,
                            interpro_references
                        })
                    } else {
                        Some(ProtInformation::Default {
                            peptide: item.sequence.clone(),
                            uniprot_id: protein.uniprot_accession.clone(),
                            protein_name: uniprot_entry.name.clone(),
                            taxon_id: uniprot_entry.taxon_id,
                            protein: uniprot_entry.protein.clone()
                        })
                    }
                })
                .collect::<Vec<ProtInformation>>()
        })
        .collect())
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<ProtInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
