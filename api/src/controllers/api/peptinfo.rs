use std::collections::HashMap;

use axum::{Json, extract::State};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{
            default_cutoff, default_domains, default_equate_il, default_extra, default_names, default_validate_taxa
        },
        generate_handlers,
        request::Flag
    },
    errors::ApiError,
    helpers::{
        distinct_peptides,
        ec_helper::{EcNumber, ec_numbers_from_map},
        fa_helper::calculate_fa,
        go_helper::{GoTerms, go_terms_from_map},
        interpro_helper::{InterproEntries, interpro_entries_from_map},
        laid_over_input,
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
    #[serde(default = "default_domains")]
    domains: Flag,
    #[serde(default = "default_names")]
    names: Flag,
    #[serde(default = "default_validate_taxa")]
    validate_taxa: Flag,
    #[serde(default = "default_cutoff")]
    cutoff: usize
}

#[derive(Serialize, Clone)]
pub struct PeptInformation {
    peptide: String,
    cutoff_used: bool,
    total_protein_count: usize,
    ec: Vec<EcNumber>,
    go: GoTerms,
    ipr: InterproEntries,
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
        domains: Flag(domains),
        names: Flag(names),
        validate_taxa: Flag(validate_taxa),
        cutoff
    }: Parameters
) -> Result<Vec<PeptInformation>, ApiError> {
    let input = sanitize_peptides(input);
    let distinct = distinct_peptides(&input);

    let result = tokio::task::block_in_place(|| index.analyse(&distinct, equate_il, false, Some(cutoff)));

    let ec_store = datastore.ec_store();
    let go_store = datastore.go_store();
    let interpro_store = datastore.interpro_store();
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let rows: HashMap<&str, Vec<PeptInformation>> = result
        .iter()
        .filter_map(|item| {
            let fa = calculate_fa(&item.proteins);

            let total_protein_count = item.proteins.len();
            let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);
            let gos = go_terms_from_map(&fa.data, go_store, extra, domains);
            let iprs = interpro_entries_from_map(&fa.data, interpro_store, extra, domains);

            // One taxon per distinct taxon, not one per matching protein. Collecting the lineages
            // is nearly the whole cost of the call, and repeats cannot change what it answers.
            let lca = calculate_lca(
                item.proteins.iter().map(|protein| protein.taxon).unique(),
                taxon_store,
                lineage_store,
                validate_taxa
            );
            let (name, rank, _) = taxon_store.get(lca as u32)?;
            let lineage = lineage_for(lca as u32, extra, names, lineage_store, taxon_store);

            Some((item.sequence, vec![PeptInformation {
                peptide: item.sequence.to_string(),
                cutoff_used: item.cutoff_used,
                total_protein_count,
                ec: ecs,
                go: gos,
                ipr: iprs,
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
    ) -> Result<Json<Vec<PeptInformation>>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
