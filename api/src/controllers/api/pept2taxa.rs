use std::collections::HashMap;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_compact, default_cutoff, default_equate_il, default_extra, default_names, default_tryptic},
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{
        distinct_peptides, laid_over_input,
        lineage_helper::{LineageResponse, Taxon, lineage_for},
        sanitize_peptides
    }
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: Flag,
    #[serde(default = "default_extra")]
    extra: Flag,
    #[serde(default = "default_names")]
    names: Flag,
    #[serde(default = "default_tryptic")]
    tryptic: Flag,
    #[serde(default = "default_compact")]
    compact: Flag,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum TaxaInformation {
    Dense(DenseTaxaInformation),
    Compact(CompactTaxaInformation)
}

#[derive(Serialize, Clone)]
pub struct DenseTaxaInformation {
    peptide: String,
    cutoff_used: bool,
    #[serde(flatten)]
    taxon: Taxon,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<LineageResponse>
}

#[derive(Serialize, Clone)]
pub struct CompactTaxaInformation {
    peptide: String,
    cutoff_used: bool,
    taxa: Vec<u32>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters {
        input,
        equate_il: Flag(equate_il),
        extra: Flag(extra),
        names: Flag(names),
        tryptic: Flag(tryptic),
        compact: Flag(compact),
        cutoff
    }: Parameters
) -> Result<Vec<TaxaInformation>, ApiError> {
    let input = sanitize_peptides(input);
    let distinct = distinct_peptides(&input);

    // Neither shape reads anything but the taxon, and `taxa` is already distinct and ascending.
    let result = tokio::task::block_in_place(|| index.analyse_taxa(&distinct, equate_il, tryptic, Some(cutoff)));

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    if compact {
        let rows: HashMap<&str, Vec<TaxaInformation>> = result
            .iter()
            .filter_map(|item| {
                let item_taxa: Vec<u32> =
                    item.taxa.iter().copied().filter(|&taxon_id| taxon_store.is_valid(taxon_id)).collect();

                if item_taxa.is_empty() {
                    return None;
                }

                Some((item.sequence, vec![TaxaInformation::Compact(CompactTaxaInformation {
                    peptide: item.sequence.to_string(),
                    cutoff_used: item.cutoff_used,
                    taxa: item_taxa
                })]))
            })
            .collect();

        return Ok(laid_over_input(&input, rows));
    }

    // A row per taxon: a repeated peptide carries all of them to each position it occupies.
    let rows: HashMap<&str, Vec<TaxaInformation>> = result
        .iter()
        .map(|item| {
            let (sequence, cutoff_used) = (item.sequence, item.cutoff_used);
            let taxa = item
                .taxa
                .iter()
                .filter_map(|&taxon| {
                    let lineage = lineage_for(taxon, extra, names, lineage_store, taxon_store);

                    Some(TaxaInformation::Dense(DenseTaxaInformation {
                        peptide: sequence.to_string(),
                        cutoff_used,
                        taxon: Taxon::new(taxon, taxon_store)?,
                        lineage
                    }))
                })
                .collect();

            (sequence, taxa)
        })
        .collect();

    Ok(laid_over_input(&input, rows))
}

generate_handlers! (
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<TaxaInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
