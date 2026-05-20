use std::collections::HashSet;
use axum::{extract::State, Json};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use index::{ProteinInfo, SearchResult};
use datastore::LineageStore;
use crate::{
    controllers::{generate_handlers, mpa::default_equate_il, mpa::default_tryptic, mpa::default_report_taxa, api::default_cutoff, api::default_validate_taxa},
    helpers::{
        fa_helper::{calculate_fa, FunctionalAggregation},
        lca_helper::calculate_lca,
        lineage_helper::{get_lineage_array, LineageVersion}
    },
    AppState
};
use crate::errors::ApiError;
use crate::helpers::filters::crap_filter::CrapFilter;
use crate::helpers::filters::empty_filter::EmptyFilter;
use crate::helpers::filters::protein_filter::ProteinFilter;
use crate::helpers::filters::proteome_filter::ProteomeFilter;
use crate::helpers::filters::taxa_filter::TaxaFilter;
use crate::helpers::filters::UniprotFilter;
use crate::helpers::sanitize_peptides;

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
    #[serde(default)]
    taxa_rank: Option<String>,
    #[serde(default = "default_validate_taxa")]
    validate_taxa: bool,
    filter: Option<Filter>,
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
    crap_filtered: bool,
}

#[derive(Serialize)]
pub struct Data {
    peptides: Vec<DataItem>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { mut peptides, equate_il, tryptic, cutoff, report_taxa, taxa_rank, validate_taxa, filter }: Parameters
) -> Result<Data, ApiError> {
    if peptides.is_empty() {
        return Ok(Data { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();

    let peptides = sanitize_peptides(peptides);
    let result = tokio::task::spawn_blocking(move || {
        index.analyse(&peptides, equate_il, tryptic, Some(cutoff))
    }).await?;

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
        },
        Some(Filter::Proteomes(proteomes)) => {
            Box::new(ProteomeFilter::new(proteomes, proteome_store).await.unwrap())
        },
        Some(Filter::Proteins(proteins)) => {
            Box::new(ProteinFilter::new(proteins))
        },
        None => Box::new(EmptyFilter::new())
    };

    let crap_filter = CrapFilter::new();
    let taxa_rank_idx = taxa_rank.as_ref().and_then(|rank| LineageStore::rank_to_idx(rank.to_lowercase().as_str()));

    Ok(Data {
        peptides: result
            .into_iter()
            .filter_map(|SearchResult { proteins, sequence, cutoff_used }| {
                let filtered_proteins: Vec<ProteinInfo> = proteins
                    .into_iter()
                    .filter(|protein| filter_proteins.filter(protein))
                    .collect();

                if filtered_proteins.is_empty() {
                    return None;
                }

                let crap_filtered = filtered_proteins.iter().any(|p| crap_filter.filter(p));

                let taxa: Vec<u32> = filtered_proteins.iter().map(|protein| protein.taxon).unique().collect();

                let taxa_at_rank: Option<Vec<u32>> = if report_taxa {
                    match taxa_rank_idx {
                        Some(idx) => Some(taxa
                            .iter()
                            .filter_map(|taxon_id| {
                                let lineage = get_lineage_array(*taxon_id, LineageVersion::V2, lineage_store);
                                lineage.get(idx).and_then(|taxon| *taxon).map(|taxon_id| taxon_id as u32)
                            })
                            .unique()
                            .collect()),
                        None => Some(taxa.clone())
                    }
                } else {
                    None
                };

                println!("taxa_rank_idx: {:?}", taxa_rank_idx);
                println!("taxa: {:?}", taxa);
                println!("taxa_at_rank: {:?}", taxa_at_rank);

                let lca = calculate_lca(
                    taxa.clone(),
                    LineageVersion::V2,
                    taxon_store,
                    lineage_store,
                    validate_taxa
                );
                let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);

                Some(DataItem {
                    sequence,
                    cutoff_used,
                    lca: Some(lca as u32),
                    lineage,
                    fa: calculate_fa(&filtered_proteins),
                    taxa: taxa_at_rank,
                    crap_filtered,
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
