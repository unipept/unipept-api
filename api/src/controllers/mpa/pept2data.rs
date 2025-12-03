use std::collections::HashSet;
use axum::{extract::State, Json};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use index::{ProteinInfo, SearchResult};
use crate::{
    controllers::{generate_handlers, mpa::default_equate_il, mpa::default_tryptic, mpa::default_report_taxa, mpa::default_blacklist_crap, api::default_cutoff},
    helpers::{
        fa_helper::{calculate_fa, FunctionalAggregation},
        lca_helper::calculate_lca,
        lineage_helper::{get_lineage_array, LineageVersion}
    },
    AppState
};
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
    #[serde(default = "default_blacklist_crap")]
    blacklist_crap: bool,
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
    lca: Option<u32>,
    lineage: Vec<Option<i32>>,
    fa: FunctionalAggregation,
    #[serde(skip_serializing_if = "Option::is_none")]
    taxa: Option<Vec<u32>>
}

#[derive(Serialize)]
pub struct Data {
    peptides: Vec<DataItem>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { mut peptides, equate_il, tryptic, cutoff, report_taxa, blacklist_crap, filter }: Parameters
) -> Result<Data, ()> {
    if peptides.is_empty() {
        return Ok(Data { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();

    let peptides = sanitize_peptides(peptides);
    let result = index.analyse(&peptides, equate_il, tryptic, Some(cutoff));

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

    let crap_blacklist = if blacklist_crap {
        Some(CrapFilter::new())
    } else {
        None
    };

    Ok(Data {
        peptides: result
            .into_iter()
            .filter_map(|SearchResult { proteins, sequence, .. }| {
                let filtered_proteins: Vec<ProteinInfo> = proteins
                    .into_iter()
                    .filter(|protein| filter_proteins.filter(protein))
                    .collect();

                if filtered_proteins.is_empty() {
                    return None;
                }

                // Remove all peptide results when any protein is in the crap blacklist
                if let Some(ref filter) = crap_blacklist {
                    if filtered_proteins.iter().any(|p| filter.filter(p)) {
                        return None;
                    }
                }

                let taxa: Vec<u32> = filtered_proteins.iter().map(|protein| protein.taxon).unique().collect();

                let lca = calculate_lca(
                    taxa.clone(),
                    LineageVersion::V2,
                    taxon_store,
                    lineage_store
                );
                let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);

                Some(DataItem {
                    sequence,
                    lca: Some(lca as u32),
                    lineage,
                    fa: calculate_fa(&filtered_proteins),
                    taxa: if report_taxa { Some(taxa) } else { None }
                })
            })
            .collect()
    })
}

generate_handlers!(
    async fn json_handler(
        state=> State<AppState>,
        params => Parameters
    ) -> Result<Json<Data>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
