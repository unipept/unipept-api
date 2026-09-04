use std::collections::HashSet;

use axum::{Json, extract::State};
use index::{ProteinInfo, SearchResult};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::{default_cutoff, default_validate_taxa},
        generate_handlers,
        mpa::{default_equate_il, default_report_taxa, default_tryptic}
    },
    errors::ApiError,
    helpers::{
        fa_helper::{FunctionalAggregation, calculate_fa},
        filters::{
            UniprotFilter, crap_filter::CrapFilter, empty_filter::EmptyFilter, protein_filter::ProteinFilter,
            proteome_filter::ProteomeFilter, taxa_filter::TaxaFilter
        },
        lca_helper::calculate_lca,
        lineage_helper::{LineageVersion, get_lineage_array},
        sanitize_peptides
    }
};

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    peptides: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_tryptic")]
    tryptic: bool,
    #[serde(default = "default_cutoff")]
    cutoff: usize,
    #[serde(default = "default_report_taxa")]
    report_taxa: bool,
    #[serde(default = "default_validate_taxa")]
    validate_taxa: bool,
    filter: Option<Filter>
}

#[derive(Deserialize)]
pub enum Filter {
    #[serde(rename = "taxa")]
    Taxa(HashSet<u32>),
    #[serde(rename = "proteomes")]
    Proteomes(HashSet<String>),
    #[serde(rename = "proteins")]
    Proteins(HashSet<String>)
}

#[derive(Serialize)]
pub struct DataItem {
    sequence: String,
    cutoff_used: bool,
    lca: Option<u32>,
    lineage: Vec<Option<i32>>,
    fa: FunctionalAggregation,
    #[serde(skip_serializing_if = "Option::is_none")]
    taxa: Option<Vec<u32>>,
    crap_filtered: bool
}

#[derive(Serialize)]
pub struct Data {
    peptides: Vec<DataItem>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters {
        mut peptides,
        equate_il,
        tryptic,
        cutoff,
        report_taxa,
        validate_taxa,
        filter
    }: Parameters
) -> Result<Data, ApiError> {
    if peptides.is_empty() {
        return Ok(Data { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();

    let peptides = sanitize_peptides(peptides);

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();
    let proteome_store = datastore.reference_proteome_store();

    let filter_proteins: Box<dyn UniprotFilter> = match filter {
        Some(Filter::Taxa(taxa)) => {
            if taxa.contains(&1) {
                Box::new(EmptyFilter::new())
            } else {
                Box::new(TaxaFilter::new(taxa, lineage_store))
            }
        }
        Some(Filter::Proteomes(proteomes)) => Box::new(ProteomeFilter::new(proteomes, proteome_store).await.unwrap()),
        Some(Filter::Proteins(proteins)) => Box::new(ProteinFilter::new(proteins)),
        None => Box::new(EmptyFilter::new())
    };

    let crap_filter = CrapFilter::new();

    // Built before the search only so a failure to construct the filters costs nothing; the
    // results may safely be held across an `.await`. `block_in_place` does require a
    // multi-threaded runtime, so a `#[tokio::test]` covering this handler needs
    // `flavor = "multi_thread"`.
    let result = tokio::task::block_in_place(|| index.analyse(&peptides, equate_il, tryptic, Some(cutoff)));

    Ok(Data {
        peptides: result
            .into_iter()
            .filter_map(|SearchResult { proteins, sequence, cutoff_used }| {
                let filtered_proteins: Vec<ProteinInfo> =
                    proteins.into_iter().filter(|protein| filter_proteins.filter(protein)).collect();

                if filtered_proteins.is_empty() {
                    return None;
                }

                let crap_filtered = filtered_proteins.iter().any(|p| crap_filter.filter(p));

                let taxa: Vec<u32> = filtered_proteins.iter().map(|protein| protein.taxon).unique().collect();

                let lca = calculate_lca(taxa.clone(), LineageVersion::V2, taxon_store, lineage_store, validate_taxa);
                let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);

                Some(DataItem {
                    sequence: sequence.to_string(),
                    cutoff_used,
                    lca: Some(lca as u32),
                    lineage,
                    fa: calculate_fa(&filtered_proteins),
                    taxa: if report_taxa { Some(taxa) } else { None },
                    crap_filtered
                })
            })
            .collect()
    })
}

generate_handlers!(
    async fn json_handler(
        state=> State<AppState>,
        params => Parameters
    ) -> Result<Json<Data>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);
