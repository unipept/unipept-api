use axum::{extract::State, Json};
use database::get_accessions_map;
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    helpers::{
        lca_helper::calculate_lca,
        lineage_helper::{get_lineage_array, LineageVersion}
    },
    AppState
};

#[derive(Deserialize)]
pub struct Parameters {
    peptide: String,
    equate_il: bool
}

#[derive(Serialize)]
pub struct ProteinInformation {
    lca: i32,
    common_lineage: Vec<i32>,
    proteins: Vec<Protein>
}

#[derive(Serialize)]
pub struct Protein {
    #[serde(rename = "uniprotAccessionId")]
    uniprot_accession_id: String,
    name: String,
    organism: u32,
    #[serde(rename = "ecNumbers")]
    ec_numbers: Vec<String>,
    #[serde(rename = "goTerms")]
    go_terms: Vec<String>,
    #[serde(rename = "interproEntries")]
    interpro_entries: Vec<String>
}

impl Default for ProteinInformation {
    fn default() -> Self {
        ProteinInformation { lca: -1, common_lineage: vec![], proteins: vec![] }
    }
}

async fn handler(
    State(AppState { index, datastore, database }): State<AppState>,
    Parameters { peptide, equate_il }: Parameters
) -> Result<ProteinInformation, ApiError> {
    let connection = database.get_conn().await?;

    let result = index.analyse(&vec![peptide], equate_il, None);

    if result.is_empty() {
        return Ok(ProteinInformation::default());
    }

    let accession_numbers: Vec<String> =
        result[0].proteins.iter().map(|protein| protein.uniprot_id.clone()).collect();

    let accessions_map = connection.interact(move |conn| get_accessions_map(conn, &accession_numbers)).await??;

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let taxa = result[0].proteins.iter().map(|protein| protein.taxon_id).collect();
    let lca = calculate_lca(taxa, LineageVersion::V2, taxon_store, lineage_store);

    let common_lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store)
        .iter()
        .filter_map(|taxon_id| *taxon_id)
        .collect::<Vec<i32>>();

    Ok(ProteinInformation {
        lca,
        common_lineage,
        proteins: result[0]
            .proteins
            .iter()
            .filter_map(|protein| {
                let uniprot_entry = accessions_map.get(&protein.uniprot_id)?;

                let fa: Vec<&str> = uniprot_entry.fa.split(';').collect();
                let ec_numbers =
                    fa.iter().filter(|key| key.starts_with("EC:")).map(ToString::to_string).collect::<Vec<String>>();
                let go_terms =
                    fa.iter().filter(|key| key.starts_with("GO:")).map(ToString::to_string).collect::<Vec<String>>();
                let interpro_entries = fa
                    .iter()
                    .filter(|key| key.starts_with("IPR:"))
                    .map(|k| k[4..].to_string())
                    .collect::<Vec<String>>();

                Some(Protein {
                    uniprot_accession_id: protein.uniprot_id.clone(),
                    name: uniprot_entry.name.clone(),
                    organism: uniprot_entry.taxon_id,
                    ec_numbers,
                    go_terms,
                    interpro_entries
                })
            })
            .collect()
    })
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<ProteinInformation>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
