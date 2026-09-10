use std::collections::HashMap;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_cutoff, default_equate_il, default_extra, default_names, default_validate_taxa},
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{
        distinct_peptides, laid_over_input,
        lca_helper::calculate_lca,
        lineage_helper::{LineageResponse, lineage_for},
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
    #[serde(default = "default_validate_taxa")]
    validate_taxa: Flag,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize, Clone)]
pub struct LcaInformation {
    peptide: String,
    cutoff_used: bool,
    #[serde(flatten)]
    taxon: Taxon,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<LineageResponse>
}

#[derive(Serialize, Clone)]
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
        validate_taxa: Flag(validate_taxa),
        cutoff
    }: Parameters
) -> Result<Vec<LcaInformation>, ApiError> {
    let input = sanitize_peptides(input);
    let distinct = distinct_peptides(&input);

    // Only the taxa are used below, so this takes the lightweight path: no accession or
    // annotation is retrieved for hits that would immediately be discarded.
    let result = tokio::task::block_in_place(|| index.analyse_taxa(&distinct, equate_il, false, Some(cutoff)));

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let rows: HashMap<&str, Vec<LcaInformation>> = result
        .iter()
        .filter_map(|item| {
            // Already sorted and deduplicated; `calculate_lca` reduces rank by rank and is
            // unaffected by repeats.
            let lca = calculate_lca(item.taxa.iter().copied(), taxon_store, lineage_store, validate_taxa);

            let (name, rank, _) = taxon_store.get(lca as u32)?;
            let lineage = lineage_for(lca as u32, extra, names, lineage_store, taxon_store);

            Some((item.sequence, vec![LcaInformation {
                peptide: item.sequence.to_string(),
                cutoff_used: item.cutoff_used,
                taxon: Taxon {
                    taxon_id: lca as u32,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.to_string()
                },
                lineage
            }]))
        })
        .collect();

    Ok(laid_over_input(&input, rows))
}

generate_handlers! (
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<LcaInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
