use std::collections::HashMap;

use axum::{Json, extract::State};
use database::get_accessions_map;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_cutoff, default_equate_il, default_extra, default_tryptic},
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{distinct_peptides, laid_over_input, sanitize_peptides}
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: Flag,
    #[serde(default = "default_extra")]
    extra: Flag,
    #[serde(default = "default_tryptic")]
    tryptic: Flag,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum ProtInformation {
    Default {
        peptide: String,
        cutoff_used: bool,
        uniprot_id: String,
        protein_name: String,
        taxon_id: u32,
        protein: String
    },
    Extra {
        peptide: String,
        cutoff_used: bool,
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
    Parameters {
        input,
        equate_il: Flag(equate_il),
        extra: Flag(extra),
        tryptic: Flag(tryptic),
        cutoff
    }: Parameters
) -> Result<Vec<ProtInformation>, ApiError> {
    let input = sanitize_peptides(input);

    let connection = database.get_conn();

    let distinct = distinct_peptides(&input);

    let result = tokio::task::block_in_place(|| index.analyse(&distinct, equate_il, tryptic, Some(cutoff)));

    // Only ever read as a lookup, so the order does not reach the answer; deduplicated so the
    // database is not asked for one accession twice.
    //
    // Deduplicated before the accessions are owned rather than after: a peptide reaches the same
    // protein through many hits, and only the distinct ones are worth an allocation.
    let accession_numbers: Vec<String> = result
        .iter()
        .flat_map(|item| item.proteins.iter().map(|protein| protein.uniprot_accession))
        .unique()
        .map(str::to_string)
        .collect();

    if accession_numbers.is_empty() {
        return Ok(vec![]);
    }

    let accessions_map = get_accessions_map(connection, &accession_numbers).await?;

    let taxon_store = datastore.taxon_store();

    // A row per protein. The cluster lookup above is keyed on accession, so the distinct peptides
    // reach the same proteins the input did.
    let rows: HashMap<&str, Vec<ProtInformation>> = result
        .iter()
        .map(|item| {
            let (sequence, cutoff_used) = (item.sequence, item.cutoff_used);
            let proteins = item
                .proteins
                .iter()
                .filter_map(|protein| {
                    let uniprot_entry = accessions_map.get(protein.uniprot_accession)?;

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
                            peptide: sequence.to_string(),
                            cutoff_used,
                            uniprot_id: protein.uniprot_accession.to_string(),
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
                            peptide: sequence.to_string(),
                            cutoff_used,
                            uniprot_id: protein.uniprot_accession.to_string(),
                            protein_name: uniprot_entry.name.clone(),
                            taxon_id: uniprot_entry.taxon_id,
                            protein: uniprot_entry.protein.clone()
                        })
                    }
                })
                .collect();

            (sequence, proteins)
        })
        .collect();

    Ok(laid_over_input(&input, rows))
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<ProtInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
