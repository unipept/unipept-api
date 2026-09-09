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
        lineage_helper::{
            Lineage,
            LineageVersion::{self, *},
            get_lineage, get_lineage_with_names
        },
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

#[allow(clippy::large_enum_variant)]
#[derive(Serialize)]
#[serde(untagged)]
pub enum TaxaInformation {
    Dense(DenseTaxaInformation),
    Compact(CompactTaxaInformation)
}

#[derive(Serialize)]
pub struct DenseTaxaInformation {
    peptide: String,
    cutoff_used: bool,
    #[serde(flatten)]
    taxon: Taxon,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<Lineage>
}

#[derive(Serialize)]
pub struct CompactTaxaInformation {
    peptide: String,
    cutoff_used: bool,
    taxa: Vec<u32>
}

#[derive(Serialize)]
pub struct Taxon {
    taxon_id: u32,
    taxon_name: String,
    taxon_rank: String
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
    }: Parameters,
    version: LineageVersion
) -> Result<Vec<TaxaInformation>, ApiError> {
    let input = sanitize_peptides(input);
    // Neither shape reads anything but the taxon, and `taxa` is already distinct and ascending.
    let result = tokio::task::block_in_place(|| index.analyse_taxa(&input, equate_il, tryptic, Some(cutoff)));

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    if compact {
        return Ok(result
            .into_iter()
            .filter_map(|item| {
                let item_taxa: Vec<u32> =
                    item.taxa.into_iter().filter(|&taxon_id| taxon_store.is_valid(taxon_id)).collect();

                if item_taxa.is_empty() {
                    return None;
                }

                Some(TaxaInformation::Compact(CompactTaxaInformation {
                    peptide: item.sequence.to_string(),
                    cutoff_used: item.cutoff_used,
                    taxa: item_taxa
                }))
            })
            .collect());
    }

    Ok(result
        .into_iter()
        .flat_map(|item| {
            let (sequence, cutoff_used) = (item.sequence, item.cutoff_used);
            item.taxa.into_iter().filter_map(move |taxon| {
                let (name, rank, _) = taxon_store.get(taxon)?;
                let lineage = match (extra, names) {
                    (true, true) => get_lineage_with_names(taxon, version, lineage_store, taxon_store),
                    (true, false) => get_lineage(taxon, version, lineage_store),
                    (false, _) => None
                };

                Some(TaxaInformation::Dense(DenseTaxaInformation {
                    peptide: sequence.to_string(),
                    cutoff_used,
                    taxon: Taxon {
                        taxon_id: taxon,
                        taxon_name: name.to_string(),
                        taxon_rank: (*rank).into()
                    },
                    lineage
                }))
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
    ) -> Result<Json<Vec<TaxaInformation>>, ApiError> {
        Ok(Json(handler(state, params, version).await?))
    }
);
